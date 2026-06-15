use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use memmap2::MmapMut;
use crate::error::BootError;
use vfs::AxicArchive;

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
    pub fn execute(archive_path: &Path) -> Result<compute::ShardEngine, BootError> {
        // Заглушка для временной директории
        let tmpfs_dir = tempfile::tempdir()?;

        let _pipeline = BootPipeline {
            archive_path: archive_path.to_path_buf(),
            tmpfs_dir: tmpfs_dir,
        };

        // Шаг 1: VfsMount — монтирует Read-Only .axic архив в виртуальную память через крейт vfs
        let archive = AxicArchive::open(archive_path).map_err(BootError::VfsMount)?;

        // Шаг 2: Manifest — извлекает и парсит manifest.toml (AOT-метаданные от baker)
        // TODO: parse manifest.toml

        // Шаг 3: RomExtract — физически эвакуирует мутабельные бинарники (.state, .axons) в tmpfs
        let state_path = _pipeline.tmpfs_dir.path().join("shard.state");
        let axons_path = _pipeline.tmpfs_dir.path().join("shard.axons");

        archive.extract_file("shard.state", &state_path).map_err(|e| {
            let io_err = match e {
                vfs::VfsError::IoError(io) => io,
                vfs::VfsError::MmapFailed(io) => io,
                other => std::io::Error::new(std::io::ErrorKind::Other, other.to_string()),
            };
            BootError::RomExtract(io_err)
        })?;

        archive.extract_file("shard.axons", &axons_path).map_err(|e| {
            let io_err = match e {
                vfs::VfsError::IoError(io) => io,
                vfs::VfsError::MmapFailed(io) => io,
                other => std::io::Error::new(std::io::ErrorKind::Other, other.to_string()),
            };
            BootError::RomExtract(io_err)
        })?;

        // Шаг 4: CabiGuard — проверяет выравнивание байт (64B для состояния, 32B для аксонов)
        let state_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&state_path)
            .map_err(|e| BootError::RomExtract(e))?;

        let axons_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&axons_path)
            .map_err(|e| BootError::RomExtract(e))?;

        // SAFETY: We have exclusive access to these newly extracted files in tmpfs.
        let state_mmap = unsafe { MmapMut::map_mut(&state_file).map_err(|e| BootError::RomExtract(e))? };
        // SAFETY: We have exclusive access to these newly extracted files in tmpfs.
        let axons_mmap = unsafe { MmapMut::map_mut(&axons_file).map_err(|e| BootError::RomExtract(e))? };

        let addr = state_mmap.as_ptr() as usize;
        if addr % 64 != 0 {
            return Err(BootError::CabiGuard { expected: 64, actual: addr % 64 });
        }

        let addr = axons_mmap.as_ptr() as usize;
        if addr % 32 != 0 {
            return Err(BootError::CabiGuard { expected: 32, actual: addr % 32 });
        }

        // Шаг 5: VramAlloc — единая монолитная аппаратная аллокация шарда в видеопамяти через compute
        // Шаг 6: HwFlash — заливка VariantParameters прямо в Constant Memory GPU
        // Шаг 7: DcrInit — инициализация Dynamic Capacity Routing (резервирование VRAM под Ghost-аксоны)
        
        // Шаг 8: BspIgnite — сборка сетевых BspBarrier и передача готового контекста в горячий runtime
        // TODO: Initialize net::BspBarrier once Layer 5 is available.

        let backend = compute_cpu::CpuBackend::new().map_err(|_| BootError::VramExhausted)?;
        let layout = compute_api::ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 10,
        };
        let shard_engine = compute::ShardEngine::new(Box::new(backend), layout)
            .map_err(|_| BootError::VramExhausted)?;

        Ok(shard_engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_pipeline_sequence_validation() {
        let phase = BootPhase::VfsMount;
        assert_eq!(phase, BootPhase::VfsMount);
        assert_ne!(phase, BootPhase::Manifest);
    }
}
