//! Scaffold implementation of CudaBackend for AxiEngine Layer 3.

use std::marker::PhantomData;
use std::rc::Rc;

use compute_api::{BackendCapabilities, BackendKind, ComputeApiError, ComputeBackend};

#[cfg(feature = "native")]
mod native;

mod resource;

use resource::ResourceRegistry;

/// Configuration parameters for the CudaBackend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CudaBackendConfig {
    /// Target NVIDIA GPU device index.
    pub device_id: u32,
}

/// A CUDA-accelerated compute backend.
///
/// Thread-affine: statically restricted to a single OS thread.
pub struct CudaBackend {
    _config: CudaBackendConfig,
    registry: ResourceRegistry,
    // Statically prevent Send and Sync
    _marker: PhantomData<Rc<()>>,
}

impl CudaBackend {
    /// Creates a new instance of the CUDA compute backend.
    ///
    /// # Errors
    /// Returns `ComputeApiError::UnsupportedBackend` in Stage 1A when native drivers/features are absent.
    pub fn new(config: CudaBackendConfig) -> Result<Self, ComputeApiError> {
        #[cfg(not(feature = "native"))]
        {
            let _ = config;
            Err(ComputeApiError::UnsupportedBackend)
        }
        #[cfg(feature = "native")]
        {
            let res = unsafe { native::axi_cuda_probe_device(config.device_id) };
            if res != 0 {
                return Err(ComputeApiError::UnsupportedBackend);
            }
            Ok(Self {
                _config: config,
                registry: ResourceRegistry::default(),
                _marker: PhantomData,
            })
        }
    }

    /// Returns static capabilities of the CUDA execution backend.
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            lane_count: 32,
            supports_async: true,
            supports_ephys: false,
            max_batch_ticks: 1000,
            alignment_bytes: 64,
            pinned_host_required: true,
        }
    }

    /// Advances an axonal propagation head on the GPU.
    #[cfg(feature = "native")]
    pub fn cuda_propagate_head_for_test(head: u32, v_seg: u32) -> Result<u32, ComputeApiError> {
        let mut out = 0u32;
        let res = unsafe { native::axi_cuda_propagate_head(head, v_seg, &mut out) };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }
        Ok(out)
    }

    /// Evaluates active tail contact on the GPU.
    #[cfg(feature = "native")]
    pub fn cuda_active_tail_hit_for_test(
        head: u32,
        seg_idx: u32,
        propagation_length: u32,
    ) -> Result<bool, ComputeApiError> {
        let mut out = 0u8;
        let res = unsafe {
            native::axi_cuda_active_tail_hit(head, seg_idx, propagation_length, &mut out)
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }
        Ok(out != 0)
    }

    /// Propagation test utility to advance axons on the GPU.
    #[cfg(feature = "native")]
    pub fn propagate_uploaded_axons_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        v_seg: u32,
    ) -> Result<(), ComputeApiError> {
        if !(1..=255).contains(&v_seg) {
            return Err(ComputeApiError::InvalidBatch);
        }
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        let res = unsafe {
            native::axi_cuda_propagate_uploaded_axons(
                resource.axons_ptr,
                resource.spec.total_axons,
                v_seg,
            )
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }
        Ok(())
    }
}

impl ComputeBackend for CudaBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Cuda
    }

    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    fn alloc_shard(
        &mut self,
        spec: compute_api::ShardAllocSpec,
    ) -> Result<compute_api::VramHandle, ComputeApiError> {
        self.registry.alloc_shard(spec)
    }

    fn upload_shard(
        &mut self,
        handle: compute_api::VramHandle,
        upload: compute_api::ShardUpload<'_>,
    ) -> Result<(), ComputeApiError> {
        self.registry.upload_shard(handle, upload)
    }

    fn run_day_batch(
        &mut self,
        _handle: compute_api::VramHandle,
        _cmd: compute_api::DayBatchCmd<'_>,
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        // Stage 1D: run_day_batch implementation is deferred.
        Err(ComputeApiError::UnsupportedFeature)
    }

    fn free_shard(&mut self, handle: compute_api::VramHandle) -> Result<(), ComputeApiError> {
        self.registry.free_shard(handle)
    }

    fn debug_snapshot(
        &mut self,
        handle: compute_api::VramHandle,
        snapshot: compute_api::ShardSnapshotMut<'_>,
    ) -> Result<(), ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }
        compute_api::validation::validate_snapshot_buffers(&resource.spec, &snapshot)?;

        #[cfg(feature = "native")]
        {
            let res = unsafe {
                native::axi_cuda_copy_d2h(
                    snapshot.state_blob.as_mut_ptr(),
                    resource.state_ptr,
                    resource.state_size,
                )
            };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }

            let res = unsafe {
                native::axi_cuda_copy_d2h(
                    snapshot.axons_blob.as_mut_ptr(),
                    resource.axons_ptr,
                    resource.axons_size,
                )
            };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            Ok(())
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = snapshot;
            Err(ComputeApiError::UnsupportedBackend)
        }
    }

    fn teardown(&mut self) -> Result<(), ComputeApiError> {
        self.registry.teardown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cuda_implements_compute_backend() {
        fn assert_impl<T: ComputeBackend>() {}
        assert_impl::<CudaBackend>();
    }

    #[test]
    fn test_cuda_backend_kind_compile_surface() {
        let backend = CudaBackend {
            _config: CudaBackendConfig::default(),
            registry: ResourceRegistry::default(),
            _marker: std::marker::PhantomData,
        };
        assert_eq!(backend.kind(), BackendKind::Cuda);
    }

    #[test]
    fn test_cuda_static_capabilities() {
        let caps = CudaBackend::static_capabilities();
        assert_eq!(caps.lane_count, 32);
        assert!(caps.supports_async);
        assert!(!caps.supports_ephys);
        assert_eq!(caps.max_batch_ticks, 1000);
        assert_eq!(caps.alignment_bytes, 64);
        assert!(caps.pinned_host_required);
    }

    #[test]
    fn test_cuda_is_not_send_sync() {
        static_assertions::assert_not_impl_any!(CudaBackend: Send, Sync);
    }

    #[test]
    fn test_cuda_generated_abi_header_contains_expected_constants() {
        let header_content = include_str!(concat!(env!("OUT_DIR"), "/generated/axi_cuda_abi.h"));

        // Structure sizes and alignments
        assert!(header_content.contains(&format!(
            "#define AXI_SIZE_VariantParameters {}",
            std::mem::size_of::<layout::VariantParameters>()
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_ALIGN_VariantParameters {}",
            std::mem::align_of::<layout::VariantParameters>()
        )));

        assert!(header_content.contains(&format!(
            "#define AXI_SIZE_BurstHeads8 {}",
            std::mem::size_of::<layout::BurstHeads8>()
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_ALIGN_BurstHeads8 {}",
            std::mem::align_of::<layout::BurstHeads8>()
        )));

        assert!(header_content.contains(&format!(
            "#define AXI_SIZE_StateFileHeader {}",
            std::mem::size_of::<layout::StateFileHeader>()
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_ALIGN_StateFileHeader {}",
            std::mem::align_of::<layout::StateFileHeader>()
        )));

        assert!(header_content.contains(&format!(
            "#define AXI_SIZE_AxonsFileHeader {}",
            std::mem::size_of::<layout::AxonsFileHeader>()
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_ALIGN_AxonsFileHeader {}",
            std::mem::align_of::<layout::AxonsFileHeader>()
        )));

        assert!(header_content.contains(&format!(
            "#define AXI_SIZE_PathsFileHeader {}",
            std::mem::size_of::<layout::PathsFileHeader>()
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_ALIGN_PathsFileHeader {}",
            std::mem::align_of::<layout::PathsFileHeader>()
        )));

        assert!(header_content.contains(&format!(
            "#define AXI_SIZE_ShardVramPtrs {}",
            std::mem::size_of::<layout::ShardVramPtrs>()
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_ALIGN_ShardVramPtrs {}",
            std::mem::align_of::<layout::ShardVramPtrs>()
        )));

        // Layout constants
        assert!(header_content.contains(&format!(
            "#define AXI_CACHE_LINE_BYTES {}",
            layout::CACHE_LINE_BYTES
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_PADDED_N_ALIGNMENT {}",
            layout::PADDED_N_ALIGNMENT
        )));

        // Types and physics constants
        assert!(header_content.contains(&format!(
            "#define AXI_AXON_SENTINEL 0x{:08X}",
            types::AXON_SENTINEL
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_EMPTY_PIXEL 0x{:08X}",
            types::EMPTY_PIXEL
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_MIN_WEIGHT_LIMIT {}",
            physics::constants::MIN_WEIGHT_LIMIT
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_MAX_WEIGHT_LIMIT {}",
            physics::constants::MAX_WEIGHT_LIMIT
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_DDS_PHASE_MOD {}ULL",
            physics::constants::DDS_PHASE_MOD
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_DDS_PHASE_MASK 0x{:X}ULL",
            physics::constants::DDS_PHASE_MASK
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_DDS_SCATTER_PRIME {}ULL",
            physics::constants::DDS_SCATTER_PRIME
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_MAX_HEARTBEAT_M {}",
            physics::constants::MAX_HEARTBEAT_M
        )));
    }

    #[test]
    #[cfg(not(feature = "native"))]
    fn test_cuda_new_without_native_returns_unsupported_backend() {
        let res = CudaBackend::new(CudaBackendConfig::default());
        assert!(matches!(res, Err(ComputeApiError::UnsupportedBackend)));
    }

    #[cfg(feature = "native")]
    fn is_gpu_available() -> bool {
        unsafe { native::axi_cuda_probe_device(0) == 0 }
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_native_propagate_head() {
        if !is_gpu_available() {
            println!("CUDA GPU not available, skipping test.");
            return;
        }

        let cases = vec![
            // a) normal active
            (100, 5),
            // b) head = AXON_SENTINEL
            (types::AXON_SENTINEL, 5),
            // c) head = AXON_SENTINEL - 1, v_seg = 1
            (types::AXON_SENTINEL - 1, 1),
            // d) head = AXON_SENTINEL - 1, v_seg = 2
            (types::AXON_SENTINEL - 1, 2),
            // e) v_seg = 0
            (10, 0),
            // general active propagation and clamp limits
            (types::AXON_SENTINEL - 100, 50),
            (types::AXON_SENTINEL - 100, 150),
        ];

        for (head, v_seg) in cases {
            let gpu_res = CudaBackend::cuda_propagate_head_for_test(head, v_seg).unwrap();
            let cpu_res = physics::propagate_head(head, v_seg);
            assert_eq!(
                gpu_res, cpu_res,
                "propagate_head mismatch for head={}, v_seg={}",
                head, v_seg
            );
        }
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_native_active_tail_hit() {
        if !is_gpu_available() {
            println!("CUDA GPU not available, skipping test.");
            return;
        }

        let cases = vec![
            // a) d < propagation_length -> true
            (100, 95, 10),
            // b) d == propagation_length -> false
            (100, 90, 10),
            // c) d > propagation_length -> false
            (100, 80, 10),
            // d) head = AXON_SENTINEL -> false
            (types::AXON_SENTINEL, 10, 5),
            // e) wraparound case: head < seg_idx -> false
            (10, 20, 5),
            // wraparound case that crosses zero -> true
            (10, 5, 10),
        ];

        for (head, seg_idx, prop_len) in cases {
            let gpu_res =
                CudaBackend::cuda_active_tail_hit_for_test(head, seg_idx, prop_len).unwrap();

            let mut heads = [types::AXON_SENTINEL; 8];
            heads[0] = head;
            let cpu_res = physics::active_tail_hit(&heads, seg_idx, prop_len);

            assert_eq!(
                gpu_res, cpu_res,
                "active_tail_hit mismatch for head={}, seg_idx={}, prop_len={}",
                head, seg_idx, prop_len
            );
        }
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_alloc_shard_returns_cuda_handle() {
        if !is_gpu_available() {
            return;
        }
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();
        assert_eq!(handle.kind(), BackendKind::Cuda);
        assert_eq!(handle.generation(), 1);
        backend.free_shard(handle).unwrap();
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_upload_and_debug_snapshot_byte_exact() {
        if !is_gpu_available() {
            return;
        }
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_size =
            compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

        let mut test_state = vec![0u8; state_size];
        for (i, val) in test_state.iter_mut().enumerate() {
            *val = (i & 0xFF) as u8;
        }

        let mut test_axons = vec![0u8; axons_size];
        for (i, val) in test_axons.iter_mut().enumerate() {
            *val = ((i * 3 + 7) & 0xFF) as u8;
        }

        let const_zero_variant = layout::VariantParameters {
            threshold: 0,
            rest_potential: 0,
            leak_shift: 0,
            homeostasis_penalty: 0,
            spontaneous_firing_period_ticks: 0,
            initial_synapse_weight: 0,
            gsop_potentiation: 0,
            gsop_depression: 0,
            homeostasis_decay: 0,
            refractory_period: 0,
            synapse_refractory_period: 0,
            signal_propagation_length: 0,
            is_inhibitory: 0,
            inertia_curve: [0; 8],
            ahp_amplitude: 0,
            _pad1: [0; 6],
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
            _leak_pad: [0; 3],
            d1_affinity: 0,
            d2_affinity: 0,
            heartbeat_m: 0,
        };
        let variant_table = [const_zero_variant; layout::VARIANT_LUT_LEN];

        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };

        backend.upload_shard(handle, upload).unwrap();

        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        let snapshot = compute_api::ShardSnapshotMut {
            state_blob: &mut snap_state,
            axons_blob: &mut snap_axons,
        };

        backend.debug_snapshot(handle, snapshot).unwrap();

        assert_eq!(snap_state, test_state);
        assert_eq!(snap_axons, test_axons);

        backend.free_shard(handle).unwrap();
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_rejects_bad_upload_sizes() {
        if !is_gpu_available() {
            return;
        }
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_size =
            compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

        let test_state = vec![0u8; state_size - 1]; // bad size
        let test_axons = vec![0u8; axons_size];

        let const_zero_variant = layout::VariantParameters {
            threshold: 0,
            rest_potential: 0,
            leak_shift: 0,
            homeostasis_penalty: 0,
            spontaneous_firing_period_ticks: 0,
            initial_synapse_weight: 0,
            gsop_potentiation: 0,
            gsop_depression: 0,
            homeostasis_decay: 0,
            refractory_period: 0,
            synapse_refractory_period: 0,
            signal_propagation_length: 0,
            is_inhibitory: 0,
            inertia_curve: [0; 8],
            ahp_amplitude: 0,
            _pad1: [0; 6],
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
            _leak_pad: [0; 3],
            d1_affinity: 0,
            d2_affinity: 0,
            heartbeat_m: 0,
        };
        let variant_table = [const_zero_variant; layout::VARIANT_LUT_LEN];

        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };

        let res = backend.upload_shard(handle, upload);
        assert!(matches!(res, Err(ComputeApiError::SizeMismatch)));

        backend.free_shard(handle).unwrap();
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_rejects_foreign_invalid_freed_handles() {
        if !is_gpu_available() {
            return;
        }
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        let foreign_handle = compute_api::VramHandle::from_raw_parts(
            BackendKind::Cpu,
            handle.id(),
            handle.generation(),
        );
        let res = backend.free_shard(foreign_handle);
        assert!(matches!(res, Err(ComputeApiError::ForeignHandle)));

        backend.free_shard(handle).unwrap();
        let res_freed = backend.free_shard(handle);
        assert!(matches!(res_freed, Err(ComputeApiError::AlreadyFreed)));

        let stale_handle = compute_api::VramHandle::from_raw_parts(
            BackendKind::Cuda,
            handle.id(),
            handle.generation() + 10,
        );
        let res_stale = backend.free_shard(stale_handle);
        assert!(matches!(res_stale, Err(ComputeApiError::InvalidHandle)));
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_teardown_invalidates_existing_handles() {
        if !is_gpu_available() {
            return;
        }
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        backend.teardown().unwrap();

        let res = backend.free_shard(handle);
        assert!(matches!(res, Err(ComputeApiError::InvalidHandle)));
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_variant_table_upload_smoke() {
        if !is_gpu_available() {
            return;
        }
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_size =
            compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

        let test_state = vec![0u8; state_size];
        let test_axons = vec![0u8; axons_size];

        let const_variant = layout::VariantParameters {
            threshold: 1000,
            rest_potential: -70,
            leak_shift: 5,
            homeostasis_penalty: 2,
            spontaneous_firing_period_ticks: 10,
            initial_synapse_weight: 4000,
            gsop_potentiation: 10,
            gsop_depression: 5,
            homeostasis_decay: 1,
            refractory_period: 3,
            synapse_refractory_period: 2,
            signal_propagation_length: 5,
            is_inhibitory: 0,
            inertia_curve: [0, 1, 2, 3, 4, 5, 6, 7],
            ahp_amplitude: 2,
            _pad1: [0; 6],
            adaptive_leak_min_shift: 1,
            adaptive_leak_gain: 2,
            adaptive_mode: 0,
            _leak_pad: [0; 3],
            d1_affinity: 0,
            d2_affinity: 0,
            heartbeat_m: 0,
        };
        let mut variant_table = [const_variant; layout::VARIANT_LUT_LEN];
        variant_table[1].threshold = 2000;
        variant_table[2].rest_potential = -60;

        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };

        backend.upload_shard(handle, upload).unwrap();

        backend.free_shard(handle).unwrap();
    }

    #[test]
    fn test_cuda_invalid_alloc_spec() {
        let mut backend = CudaBackend {
            _config: CudaBackendConfig::default(),
            registry: ResourceRegistry::default(),
            _marker: std::marker::PhantomData,
        };

        // padded_n = 0 -> InvalidShape
        let spec_zero = compute_api::ShardAllocSpec {
            padded_n: 0,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let res_zero = backend.alloc_shard(spec_zero);
        assert!(matches!(res_zero, Err(ComputeApiError::InvalidShape)));

        // padded_n not aligned to 64 -> AlignmentViolation
        let spec_unaligned = compute_api::ShardAllocSpec {
            padded_n: 63,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let res_unaligned = backend.alloc_shard(spec_unaligned);
        assert!(matches!(
            res_unaligned,
            Err(ComputeApiError::AlignmentViolation)
        ));
    }

    #[test]
    fn test_run_day_batch_returns_unsupported_feature_until_stage_1d() {
        let mut backend = CudaBackend {
            _config: CudaBackendConfig::default(),
            registry: ResourceRegistry::default(),
            _marker: std::marker::PhantomData,
        };

        let handle = compute_api::VramHandle::from_raw_parts(
            BackendKind::Cuda,
            core::num::NonZeroU64::new(1).unwrap(),
            1,
        );
        let mut output_spikes = [0u32; 1];
        let mut output_spike_counts = [0u32; 1];
        let cmd = compute_api::DayBatchCmd {
            tick_base: 0,
            sync_batch_ticks: 1,
            v_seg: 1,
            dopamine: 0,
            input_words_per_tick: 0,
            max_spikes_per_tick: 1,
            num_outputs: 0,
            virtual_offset: 0,
            num_virtual_axons: 0,
            input_bitmask: None,
            incoming_spikes: None,
            incoming_spike_counts: &[0],
            mapped_soma_ids: &[],
            output_spikes: &mut output_spikes,
            output_spike_counts: &mut output_spike_counts,
        };

        let res = backend.run_day_batch(handle, cmd);
        assert!(matches!(res, Err(ComputeApiError::UnsupportedFeature)));
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_native_propagate_uploaded_axons() {
        if !is_gpu_available() {
            return;
        }
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

        // 1. Test calling on non-uploaded backend returns BackendNotInitialized
        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 3,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        let res_not_init = backend.propagate_uploaded_axons_for_test(handle, 5);
        assert!(matches!(
            res_not_init,
            Err(ComputeApiError::BackendNotInitialized)
        ));

        // 2. Upload axons and state
        let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_size =
            compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

        let test_state = vec![0u8; state_size];

        // Formulate test axons
        // Header: magic "AXAX", version 1, total_axons = 3, _padding = 0
        let header = layout::AxonsFileHeader::new(spec.total_axons);
        let mut test_axons_blob = vec![0u8; axons_size];
        test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

        // Create BurstHeads8 for 3 axons
        let mut heads = [
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
        ];

        // Populate various heads
        // Axon 0
        heads[0].h0 = 100; // normal active
        heads[0].h1 = types::AXON_SENTINEL; // sentinel (inactive)
        heads[0].h2 = types::AXON_SENTINEL - 1; // active, edge case
        heads[0].h3 = types::AXON_SENTINEL + 1; // active, value above sentinel

        // Axon 1
        heads[1].h0 = 10;
        heads[1].h1 = 20;
        heads[1].h2 = types::AXON_SENTINEL;

        // Copy heads into the blob
        let heads_bytes = bytemuck::cast_slice(&heads);
        test_axons_blob[16..16 + heads_bytes.len()].copy_from_slice(heads_bytes);

        let const_zero_variant = layout::VariantParameters {
            threshold: 0,
            rest_potential: 0,
            leak_shift: 0,
            homeostasis_penalty: 0,
            spontaneous_firing_period_ticks: 0,
            initial_synapse_weight: 0,
            gsop_potentiation: 0,
            gsop_depression: 0,
            homeostasis_decay: 0,
            refractory_period: 0,
            synapse_refractory_period: 0,
            signal_propagation_length: 0,
            is_inhibitory: 0,
            inertia_curve: [0; 8],
            ahp_amplitude: 0,
            _pad1: [0; 6],
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
            _leak_pad: [0; 3],
            d1_affinity: 0,
            d2_affinity: 0,
            heartbeat_m: 0,
        };
        let variant_table = [const_zero_variant; layout::VARIANT_LUT_LEN];

        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons_blob,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        // 3. Test invalid v_seg (0 or > 255) returns InvalidBatch
        let res_invalid_vseg0 = backend.propagate_uploaded_axons_for_test(handle, 0);
        assert!(matches!(
            res_invalid_vseg0,
            Err(ComputeApiError::InvalidBatch)
        ));
        let res_invalid_vseg256 = backend.propagate_uploaded_axons_for_test(handle, 256);
        assert!(matches!(
            res_invalid_vseg256,
            Err(ComputeApiError::InvalidBatch)
        ));

        // 4. Run propagation primitive on GPU
        let v_seg = 5;
        backend
            .propagate_uploaded_axons_for_test(handle, v_seg)
            .unwrap();

        // 5. Read back via debug_snapshot
        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons_blob = vec![0u8; axons_size];
        let snapshot = compute_api::ShardSnapshotMut {
            state_blob: &mut snap_state,
            axons_blob: &mut snap_axons_blob,
        };
        backend.debug_snapshot(handle, snapshot).unwrap();

        // 6. Compare with CPU-expected propagate_head for each head
        let mut expected_heads = heads;

        let update_head = |h: &mut u32| {
            *h = physics::propagate_head(*h, v_seg);
        };

        // Axon 0
        update_head(&mut expected_heads[0].h0);
        update_head(&mut expected_heads[0].h1);
        update_head(&mut expected_heads[0].h2);
        update_head(&mut expected_heads[0].h3);
        update_head(&mut expected_heads[0].h4);
        update_head(&mut expected_heads[0].h5);
        update_head(&mut expected_heads[0].h6);
        update_head(&mut expected_heads[0].h7);

        // Axon 1
        update_head(&mut expected_heads[1].h0);
        update_head(&mut expected_heads[1].h1);
        update_head(&mut expected_heads[1].h2);
        update_head(&mut expected_heads[1].h3);
        update_head(&mut expected_heads[1].h4);
        update_head(&mut expected_heads[1].h5);
        update_head(&mut expected_heads[1].h6);
        update_head(&mut expected_heads[1].h7);

        // Axon 2
        update_head(&mut expected_heads[2].h0);
        update_head(&mut expected_heads[2].h1);
        update_head(&mut expected_heads[2].h2);
        update_head(&mut expected_heads[2].h3);
        update_head(&mut expected_heads[2].h4);
        update_head(&mut expected_heads[2].h5);
        update_head(&mut expected_heads[2].h6);
        update_head(&mut expected_heads[2].h7);

        let mut expected_axons_blob = vec![0u8; axons_size];
        expected_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));
        expected_axons_blob[16..16 + heads_bytes.len()]
            .copy_from_slice(bytemuck::cast_slice(&expected_heads));

        assert_eq!(snap_axons_blob, expected_axons_blob);

        backend.free_shard(handle).unwrap();
    }
}
