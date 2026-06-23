use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use memmap2::MmapMut;
use crate::error::BootError;
use vfs::AxicArchive;
use config::ZoneManifest;
use compute::BackendType;
use bytemuck::Zeroable;

/// Перечисление всех 8 изолированных фаз загрузки для телеметрии и отладки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    /// Монтирование Read-Only .axic архива в виртуальную память через vfs
    VfsMount,
    /// Извлечение и парсинг manifest.toml
    Manifest,
    /// Эвакуация мутабельных бинарников во временную tmpfs
    RomExtract,
    /// Проверка выравнивания байт по границам 64 и 32 байт
    CabiGuard,
    /// Монолитная аппаратная аллокация шарда в видеопамяти
    VramAlloc,
    /// Заливка VariantParameters в Constant Memory GPU
    HwFlash,
    /// Инициализация Dynamic Capacity Routing
    DcrInit,
    /// Сборка сетевых BspBarrier
    BspIgnite,
}

/// Context for simulation node boot initialization.
///
/// ### Invariants
///
/// - **INV-BOOT-002: RAII Fail-Fast (Resource Isolation on Failures)**
///   If any phase before VRAM allocation returns an error, VRAM allocation is not performed.
///   The cleanup of OS temporary files must occur implicitly and guaranteed through the implementation
///   of the `Drop` trait for `BootPipeline` (RAII), so that no junk is left on the disk even if a panic
///   occurs inside the phases.
///
/// - **INV-BOOT-005: RAM-Disk Mutability (SSD Wear Protection)**
///   Mutable files (`.state` and `.axons`) extracted from the archive for runtime operations
///   must be unpacked strictly into a directory mounted in the OS RAM (e.g., `/dev/shm` on Linux).
///   Intensive writing of gigabytes of simulation state to physical SSD storage will lead to its rapid
///   hardware wear (Wear Leveling exhaustion).
pub struct BootPipeline {
    /// Путь к исходному Read-Only архиву `.axic`.
    pub archive_path: PathBuf,
    /// Временная директория ОС для мутабельных файлов (`.state`, `.axons`).
    /// Семантика `Drop` этого типа гарантирует физическое удаление файлов при `Err`.
    pub tmpfs_dir: TempDir,
}

impl BootPipeline {
    /// Точка входа. Создает контекст, разворачивает tmpfs и прогоняет 8 фаз загрузки.
    pub fn execute(
        archive_path: &Path,
        zone_name: Option<&str>,
        backend_type: BackendType,
    ) -> Result<(compute::ShardEngine, ZoneManifest, TempDir), BootError> {
        // Шаг 1: VfsMount — монтирует Read-Only .axic архив в виртуальную память через крейт vfs
        let archive = AxicArchive::open(archive_path).map_err(BootError::VfsMount)?;

        // Шаг 2: Manifest — извлекает и парсит manifest.toml (AOT-метаданные от baker)
        let manifest_path = if let Some(zone) = zone_name {
            format!("baked/{}/manifest.toml", zone)
        } else {
            "manifest.toml".to_string()
        };

        let manifest_bytes = archive.get_file(&manifest_path)
            .map_err(|_| BootError::Manifest(format!("{} not found in archive", manifest_path)))?;
        let manifest_str = std::str::from_utf8(manifest_bytes)
            .map_err(|e| BootError::Manifest(format!("Invalid UTF-8 in manifest: {}", e)))?;
        let manifest: ZoneManifest = toml::from_str(manifest_str)
            .map_err(|e| BootError::Manifest(format!("Failed to parse manifest: {}", e)))?;

        // Шаг 3: RomExtract — физически эвакуирует мутабельные бинарники (.state, .axons) в tmpfs
        #[cfg(target_os = "linux")]
        let tmpfs_dir = tempfile::Builder::new()
            .prefix("axicor_")
            .tempdir_in("/dev/shm")
            .map_err(BootError::RomExtract)?;

        #[cfg(not(target_os = "linux"))]
        let tmpfs_dir = tempfile::tempdir().map_err(BootError::RomExtract)?;

        // INV-BOOT-005: RAM-Disk Mutability check on Linux
        #[cfg(target_os = "linux")]
        {
            let path_str = tmpfs_dir.path().to_string_lossy();
            if !path_str.starts_with("/dev/shm") {
                return Err(BootError::Manifest("INV-BOOT-005: tmpfs path must start with /dev/shm".to_string()));
            }
        }

        let _pipeline = BootPipeline {
            archive_path: archive_path.to_path_buf(),
            tmpfs_dir,
        };

        let state_vfs_path = if let Some(zone) = zone_name {
            format!("baked/{}/shard.state", zone)
        } else {
            "shard.state".to_string()
        };
        let axons_vfs_path = if let Some(zone) = zone_name {
            format!("baked/{}/shard.axons", zone)
        } else {
            "shard.axons".to_string()
        };

        let state_path = _pipeline.tmpfs_dir.path().join("shard.state");
        let axons_path = _pipeline.tmpfs_dir.path().join("shard.axons");

        archive.extract_file(&state_vfs_path, &state_path).map_err(|e| {
            let io_err = match e {
                vfs::VfsError::IoError(io) => io,
                vfs::VfsError::MmapFailed(io) => io,
                other => std::io::Error::new(std::io::ErrorKind::Other, other.to_string()),
            };
            BootError::RomExtract(io_err)
        })?;

        archive.extract_file(&axons_vfs_path, &axons_path).map_err(|e| {
            let io_err = match e {
                vfs::VfsError::IoError(io) => io,
                vfs::VfsError::MmapFailed(io) => io,
                other => std::io::Error::new(std::io::ErrorKind::Other, other.to_string()),
            };
            BootError::RomExtract(io_err)
        })?;

        // Extract geometry, paths and BrainDNA config files if present for baker daemon and tools
        let geom_vfs_path = if let Some(zone) = zone_name {
            format!("baked/{}/shard.geom", zone)
        } else {
            "shard.geom".to_string()
        };
        let paths_vfs_path = if let Some(zone) = zone_name {
            format!("baked/{}/shard.paths", zone)
        } else {
            "shard.paths".to_string()
        };

        let _ = archive.extract_file(&geom_vfs_path, &_pipeline.tmpfs_dir.path().join("shard.geom"));
        let _ = archive.extract_file(&paths_vfs_path, &_pipeline.tmpfs_dir.path().join("shard.paths"));

        let brain_dna_dir = _pipeline.tmpfs_dir.path().join("BrainDNA");
        let _ = std::fs::create_dir_all(&brain_dna_dir);
        for file in ["simulation.toml", "blueprints.toml", "anatomy.toml", "shard.toml", "io.toml"] {
            let file_vfs_path = if let Some(zone) = zone_name {
                format!("baked/{}/BrainDNA/{}", zone, file)
            } else {
                format!("BrainDNA/{}", file)
            };
            let _ = archive.extract_file(&file_vfs_path, &brain_dna_dir.join(file));
        }

        // Шаг 4: CabiGuard — проверяет выравнивание байт (64B для состояния, 32B для аксонов)
        let state_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&state_path)
            .map_err(BootError::RomExtract)?;

        let axons_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&axons_path)
            .map_err(BootError::RomExtract)?;

        // SAFETY: We have exclusive access to these newly extracted files in tmpfs.
        let state_mmap = unsafe { MmapMut::map_mut(&state_file).map_err(BootError::RomExtract)? };
        // SAFETY: We have exclusive access to these newly extracted files in tmpfs.
        let axons_mmap = unsafe { MmapMut::map_mut(&axons_file).map_err(BootError::RomExtract)? };

        let state_addr = state_mmap.as_ptr() as usize;
        if state_addr % 64 != 0 {
            return Err(BootError::CabiGuard { expected: 64, actual: state_addr % 64 });
        }

        let axons_addr = axons_mmap.as_ptr() as usize;
        if axons_addr % 32 != 0 {
            return Err(BootError::CabiGuard { expected: 32, actual: axons_addr % 32 });
        }

        // Check size matching for safety
        let padded_n = manifest.memory.padded_n;
        let (_, expected_state_size) = layout::calculate_state_blob_size(padded_n);
        if state_mmap.len() != expected_state_size {
            return Err(BootError::Manifest(format!(
                "State file size mismatch: expected {}, got {}",
                expected_state_size, state_mmap.len()
            )));
        }

        if axons_mmap.len() % 32 != 0 {
            return Err(BootError::CabiGuard { expected: 32, actual: axons_mmap.len() % 32 });
        }

        // Шаг 5: VramAlloc — единая монолитная аппаратная аллокация шарда в видеопамяти через compute
        let backend = compute::instantiate_backend(backend_type, None)
            .map_err(|_| BootError::VramExhausted)?;

        let total_ghosts = manifest.memory.ghost_capacity;
        let file_axons = axons_mmap.len() / 32;
        let calc_axons = manifest.memory.padded_n + manifest.memory.virtual_axons + total_ghosts;
        let total_axons = std::cmp::max(calc_axons, file_axons);
        let total_axons = (total_axons + 31) & !31; // Warp Alignment

        let layout = compute_api::ShardLayout {
            padded_n: padded_n as u32,
            total_axons: total_axons as u32,
            total_ghosts: total_ghosts as u32,
        };

        let shard_engine = compute::ShardEngine::new(backend, layout)
            .map_err(|_| BootError::VramExhausted)?;

        // Upload state to VRAM/RAM
        shard_engine.upload_state(&state_mmap).map_err(|_| BootError::VramExhausted)?;

        // Шаг 6: HwFlash — заливка VariantParameters прямо в Constant Memory GPU
        let mut gpu_variants = [layout::VariantParameters::zeroed(); 16];
        for v in &manifest.variants {
            if (v.id as usize) < 16 {
                gpu_variants[v.id as usize] = v.clone().into_gpu();
            }
        }
        shard_engine.upload_variants(&gpu_variants).map_err(|_| BootError::VramExhausted)?;

        // Шаг 7: DcrInit — инициализация Dynamic Capacity Routing
        // (In this version, ShardEngine pre-allocates everything based on ShardLayout, satisfying flat allocation)

        // Шаг 8: BspIgnite — сборка сетевых BspBarrier и передача готового контекста в горячий runtime
        // Note: The BspBarrier is actually instantiated inside runtime using config and net crates.

        Ok((shard_engine, manifest, _pipeline.tmpfs_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_test_manifest(dir: &Path) -> std::io::Result<()> {
        let toml_str = r#"
magic = 1145980243
zone_hash = 12345
blueprints_path = "blueprints.toml"

[memory]
padded_n = 64
virtual_axons = 10
ghost_capacity = 5
v_seg = 2

[network]
slow_path_tcp = 9000
external_udp_in = 9001
external_udp_out = 9002
fast_path_udp_local = 9003
fast_path_peers = {}

[settings]
night_interval_ticks = 1000
save_checkpoints_interval_ticks = 5000

[settings.plasticity]
prune_threshold = 15
max_sprouts = 4

[[variants]]
id = 0
name = "Exc"
threshold = 20000
rest_potential = -70000
leak_shift = 4
homeostasis_penalty = 1000
spontaneous_firing_period_ticks = 100
initial_synapse_weight = 10
gsop_potentiation = 15
gsop_depression = 5
homeostasis_decay = 990
refractory_period = 5
synapse_refractory_period = 10
signal_propagation_length = 8
is_inhibitory = false
inertia_curve = [10, 20, 30, 40, 50, 60, 70, 80]
ahp_amplitude = 0
adaptive_leak_min_shift = -5
adaptive_leak_gain = 2
adaptive_mode = 1
d1_affinity = 80
d2_affinity = 20
"#;
        let mut file = std::fs::File::create(dir.join("manifest.toml"))?;
        file.write_all(toml_str.as_bytes())?;
        Ok(())
    }

    #[test]
    fn test_boot_pipeline_sequence_validation() {
        let phase = BootPhase::VfsMount;
        assert_eq!(phase, BootPhase::VfsMount);
        assert_ne!(phase, BootPhase::Manifest);
    }

    #[test]
    fn test_fail_fast_on_missing_manifest() {
        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("test_missing_manifest.axic");

        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        // Write only files, no manifest
        std::fs::write(src_dir.join("shard.state"), vec![0u8; 100]).unwrap();
        std::fs::write(src_dir.join("shard.axons"), vec![0u8; 32]).unwrap();

        vfs::pack_directory(&src_dir, &archive_path).unwrap();

        let res = BootPipeline::execute(&archive_path, None, BackendType::Cpu);
        assert!(res.is_err());
        let err = res.err().unwrap();
        match err {
            BootError::Manifest(msg) => assert!(msg.contains("manifest.toml not found")),
            other => panic!("Expected BootError::Manifest, got {:?}", other),
        }
    }

    #[test]
    fn test_cabi_guard_alignment_check() {
        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("test_cabi_guard.axic");

        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        write_test_manifest(&src_dir).unwrap();

        // Write misaligned axons (size not a multiple of 32 bytes)
        let (_, expected_state_size) = layout::calculate_state_blob_size(64);
        std::fs::write(src_dir.join("shard.state"), vec![0u8; expected_state_size]).unwrap();
        std::fs::write(src_dir.join("shard.axons"), vec![0u8; 15]).unwrap(); // Misaligned size

        vfs::pack_directory(&src_dir, &archive_path).unwrap();

        let res = BootPipeline::execute(&archive_path, None, BackendType::Cpu);
        assert!(res.is_err());
        let err = res.err().unwrap();
        match err {
            BootError::CabiGuard { expected: 32, .. } => {},
            other => panic!("Expected BootError::CabiGuard for axons alignment, got {:?}", other),
        }
    }

    #[test]
    fn test_full_boot_flow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("test_full_boot.axic");

        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        write_test_manifest(&src_dir).unwrap();

        let (_, expected_state_size) = layout::calculate_state_blob_size(64);
        std::fs::write(src_dir.join("shard.state"), vec![0u8; expected_state_size]).unwrap();
        std::fs::write(src_dir.join("shard.axons"), vec![0u8; 32]).unwrap();

        vfs::pack_directory(&src_dir, &archive_path).unwrap();

        let res = BootPipeline::execute(&archive_path, None, BackendType::Cpu);
        assert!(res.is_ok());
        let (engine, manifest, tmp) = res.unwrap();
        assert_eq!(manifest.zone_hash, 12345);
        assert_eq!(engine.layout().padded_n, 64);
        assert!(tmp.path().exists());
    }
}
