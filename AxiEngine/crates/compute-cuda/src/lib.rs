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

    /// Single-tick injection and propagation test utility.
    #[cfg(feature = "native")]
    pub fn inject_and_propagate_axons_tick_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        v_seg: u32,
        cmd_virtual_offset: u32,
        num_virtual_axons: u32,
        input_bitmask: Option<&[u32]>,
        incoming_spikes: Option<&[u32]>,
    ) -> Result<(), ComputeApiError> {
        if !(1..=255).contains(&v_seg) {
            return Err(ComputeApiError::InvalidBatch);
        }
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        if let Some(mask) = input_bitmask {
            if mask.len() > u32::MAX as usize {
                return Err(ComputeApiError::CapacityExceeded);
            }
            let required_words = num_virtual_axons.div_ceil(32) as usize;
            if mask.len() < required_words {
                return Err(ComputeApiError::InvalidBatch);
            }
        }
        if let Some(spikes) = incoming_spikes {
            if spikes.len() > u32::MAX as usize {
                return Err(ComputeApiError::CapacityExceeded);
            }
        }

        let mut input_words_len = 0u32;
        let mut d_bitmask = std::ptr::null_mut();
        if let Some(mask) = input_bitmask {
            if !mask.is_empty() {
                input_words_len = mask.len() as u32;
                let size = std::mem::size_of_val(mask);
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut d_bitmask) };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
                let res = unsafe {
                    native::axi_cuda_copy_h2d(d_bitmask, mask.as_ptr() as *const u8, size)
                };
                if res != 0 {
                    unsafe {
                        let _ = native::axi_cuda_free(d_bitmask);
                    }
                    return Err(native::map_cuda_error(res));
                }
            }
        }

        let mut d_spikes = std::ptr::null_mut();
        let mut spikes_count = 0u32;
        if let Some(spikes) = incoming_spikes {
            if !spikes.is_empty() {
                spikes_count = spikes.len() as u32;
                let size = std::mem::size_of_val(spikes);
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut d_spikes) };
                if res != 0 {
                    unsafe {
                        let _ = native::axi_cuda_free(d_bitmask);
                    }
                    return Err(native::map_cuda_error(res));
                }
                let res = unsafe {
                    native::axi_cuda_copy_h2d(d_spikes, spikes.as_ptr() as *const u8, size)
                };
                if res != 0 {
                    unsafe {
                        let _ = native::axi_cuda_free(d_bitmask);
                        let _ = native::axi_cuda_free(d_spikes);
                    }
                    return Err(native::map_cuda_error(res));
                }
            }
        }

        let res = unsafe {
            native::axi_cuda_inject_and_propagate_axons_tick(
                resource.axons_ptr,
                resource.spec.total_axons,
                v_seg,
                resource.spec.virtual_offset,
                cmd_virtual_offset,
                num_virtual_axons,
                d_bitmask as *const u32,
                input_words_len,
                d_spikes as *const u32,
                spikes_count,
            )
        };

        unsafe {
            let _ = native::axi_cuda_free(d_bitmask);
            let _ = native::axi_cuda_free(d_spikes);
        }

        if res != 0 {
            return Err(native::map_cuda_error(res));
        }
        Ok(())
    }

    /// Native-only test utility for active-tail input-current probe.
    #[cfg(feature = "native")]
    pub fn compute_input_current_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        propagation_length: u32,
        out_i_in: &mut [i32],
    ) -> Result<(), ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        if out_i_in.len() > u32::MAX as usize {
            return Err(ComputeApiError::CapacityExceeded);
        }

        if out_i_in.len() < resource.spec.padded_n as usize {
            return Err(ComputeApiError::InvalidBatch);
        }

        let offsets = layout::compute_state_offsets(resource.spec.padded_n as usize);
        if offsets.off_targets > u32::MAX as usize || offsets.off_weights > u32::MAX as usize {
            return Err(ComputeApiError::CapacityExceeded);
        }

        let res = unsafe {
            native::axi_cuda_compute_input_current_probe(
                resource.state_ptr,
                resource.axons_ptr,
                resource.spec.padded_n,
                resource.spec.total_axons,
                offsets.off_targets as u32,
                offsets.off_weights as u32,
                propagation_length,
                out_i_in.as_mut_ptr(),
                out_i_in.len() as u32,
            )
        };

        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        Ok(())
    }

    /// Native-only test utility for GLIF membrane updates.
    #[cfg(feature = "native")]
    pub fn apply_glif_membrane_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        i_in: &[i32],
    ) -> Result<(), ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        if i_in.len() > u32::MAX as usize {
            return Err(ComputeApiError::CapacityExceeded);
        }

        if i_in.len() < resource.spec.padded_n as usize {
            return Err(ComputeApiError::InvalidBatch);
        }

        let offsets = layout::compute_state_offsets(resource.spec.padded_n as usize);
        if offsets.off_voltage > u32::MAX as usize
            || offsets.off_flags > u32::MAX as usize
            || offsets.off_thresh > u32::MAX as usize
            || offsets.off_timers > u32::MAX as usize
        {
            return Err(ComputeApiError::CapacityExceeded);
        }

        // Re-upload this resource's variant_table to device constant memory
        let variant_bytes = resource.variant_table.as_ptr() as *const u8;
        let variant_size = std::mem::size_of_val(&resource.variant_table);
        let upload_res =
            unsafe { native::axi_cuda_upload_variant_table(variant_bytes, variant_size) };
        if upload_res != 0 {
            return Err(native::map_cuda_error(upload_res));
        }

        let res = unsafe {
            native::axi_cuda_apply_glif_membrane_probe(
                resource.state_ptr,
                resource.spec.padded_n,
                offsets.off_voltage as u32,
                offsets.off_flags as u32,
                offsets.off_thresh as u32,
                offsets.off_timers as u32,
                i_in.as_ptr(),
                i_in.len() as u32,
            )
        };

        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        Ok(())
    }

    /// Native-only test utility that runs compute_input_current_probe and apply_glif_membrane_probe in one tick.
    #[cfg(feature = "native")]
    pub fn run_current_glif_tick_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        propagation_length: u32,
    ) -> Result<(), ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        let padded_n = resource.spec.padded_n as usize;
        let mut i_in = vec![0i32; padded_n];

        self.compute_input_current_probe_for_test(handle, propagation_length, &mut i_in)?;
        self.apply_glif_membrane_probe_for_test(handle, &i_in)?;

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

    #[cfg(feature = "native")]
    static GPU_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        assert!(header_content.contains(&format!(
            "#define AXI_MAX_DENDRITES {}",
            layout::MAX_DENDRITES
        )));
        assert!(header_content.contains(&format!("#define AXI_MAX_AXON_ID {}", types::MAX_AXON_ID)));
        assert!(header_content.contains(&format!(
            "#define AXI_MAX_SEGMENT_OFFSET {}",
            types::MAX_SEGMENT_OFFSET
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_MASS_TO_CHARGE_SHIFT {}",
            physics::constants::MASS_TO_CHARGE_SHIFT
        )));

        let dummy = layout::VariantParameters {
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
        let base_ptr = &dummy as *const _ as usize;
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_threshold {}",
            (&dummy.threshold as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_rest_potential {}",
            (&dummy.rest_potential as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_leak_shift {}",
            (&dummy.leak_shift as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_homeostasis_penalty {}",
            (&dummy.homeostasis_penalty as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_homeostasis_decay {}",
            (&dummy.homeostasis_decay as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_refractory_period {}",
            (&dummy.refractory_period as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_ahp_amplitude {}",
            (&dummy.ahp_amplitude as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_adaptive_leak_min_shift {}",
            (&dummy.adaptive_leak_min_shift as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_adaptive_leak_gain {}",
            (&dummy.adaptive_leak_gain as *const _ as usize) - base_ptr
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_OFFSET_VariantParameters_adaptive_mode {}",
            (&dummy.adaptive_mode as *const _ as usize) - base_ptr
        )));

        assert!(header_content.contains(&format!(
            "#define AXI_SOMA_SPIKING_MASK {}",
            types::SOMA_SPIKING_MASK
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_SOMA_BURST_MASK {}",
            types::SOMA_BURST_MASK
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_SOMA_BURST_SHIFT {}",
            types::SOMA_BURST_SHIFT
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_SOMA_TYPE_MASK {}",
            types::SOMA_TYPE_MASK
        )));
        assert!(header_content.contains(&format!(
            "#define AXI_SOMA_TYPE_SHIFT {}",
            types::SOMA_TYPE_SHIFT
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
        let _lock = GPU_TEST_LOCK.lock().unwrap();
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
        let _lock = GPU_TEST_LOCK.lock().unwrap();
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
        let _lock = GPU_TEST_LOCK.lock().unwrap();
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
        let _lock = GPU_TEST_LOCK.lock().unwrap();
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

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_native_inject_and_propagate_axons_tick() {
        if !is_gpu_available() {
            return;
        }
        let _lock = GPU_TEST_LOCK.lock().unwrap();
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

        // 1. Create a shard with 4 axons, virtual offset 100
        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 4,
            total_ghosts: 0,
            virtual_offset: 100,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        // Check non-uploaded returns BackendNotInitialized
        let res_not_init =
            backend.inject_and_propagate_axons_tick_for_test(handle, 5, 95, 10, None, None);
        assert!(matches!(
            res_not_init,
            Err(ComputeApiError::BackendNotInitialized)
        ));

        let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_size =
            compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

        // Fill initial axons with sentinels
        let header = layout::AxonsFileHeader::new(spec.total_axons);
        let mut initial_axons_blob = vec![0u8; axons_size];
        initial_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

        let mut heads = [
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
        ];

        // Seed some initial heads to verify propagation and shifting
        heads[0].h0 = 1000;
        heads[0].h1 = types::AXON_SENTINEL;
        heads[1].h0 = 2000;
        heads[2].h0 = 3000;
        heads[3].h0 = 4000;

        let heads_bytes = bytemuck::cast_slice(&heads);
        initial_axons_blob[16..16 + heads_bytes.len()].copy_from_slice(heads_bytes);

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
            state_blob: &vec![0u8; state_size],
            axons_blob: &initial_axons_blob,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        // Check invalid v_seg range checks
        let res_vseg0 =
            backend.inject_and_propagate_axons_tick_for_test(handle, 0, 95, 10, None, None);
        assert!(matches!(res_vseg0, Err(ComputeApiError::InvalidBatch)));

        let res_vseg256 =
            backend.inject_and_propagate_axons_tick_for_test(handle, 256, 95, 10, None, None);
        assert!(matches!(res_vseg256, Err(ComputeApiError::InvalidBatch)));

        // Inputs for the tick:
        // Virtual Offset: 95
        // Num Virtual Axons: 10
        // Virtual Axon range: 95..105 (global IDs)
        // bitmask: 1 word, active bits: 0, 5, 6, 7, 9
        // - bit 0 (global ID 95): active, but ignored
        // - bit 5 (global ID 100): active -> axon 0 gets 1 virtual injection
        // - bit 6 (global ID 101): active -> axon 1 gets 1 virtual injection
        // - bit 7 (global ID 102): active -> axon 2 gets 1 virtual injection
        // - bit 9 (global ID 104): active, but ignored
        let input_bitmask = vec![737];

        // Incoming spikes:
        // Spikes list: [1, 2, 2, 9] (local axon IDs)
        // - axon 1 (local ID 1) gets 1 incoming injection
        // - axon 2 (local ID 2) gets 2 incoming injections (duplicate!)
        // - local ID 9 gets ignored
        let incoming_spikes = vec![1, 2, 2, 9];

        // Let's run the tick on GPU
        let v_seg = 5;
        backend
            .inject_and_propagate_axons_tick_for_test(
                handle,
                v_seg,
                95,
                10,
                Some(&input_bitmask),
                Some(&incoming_spikes),
            )
            .unwrap();

        // Read back
        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        // Calculate expected on CPU
        let init_val = physics::initial_axon_head(v_seg);
        let mut expected_heads = heads;

        let apply_injections_and_propagate = |heads_arr: &mut layout::BurstHeads8, n: usize| {
            let mut h_slice = [
                heads_arr.h0,
                heads_arr.h1,
                heads_arr.h2,
                heads_arr.h3,
                heads_arr.h4,
                heads_arr.h5,
                heads_arr.h6,
                heads_arr.h7,
            ];
            for _ in 0..n {
                h_slice[7] = h_slice[6];
                h_slice[6] = h_slice[5];
                h_slice[5] = h_slice[4];
                h_slice[4] = h_slice[3];
                h_slice[3] = h_slice[2];
                h_slice[2] = h_slice[1];
                h_slice[1] = h_slice[0];
                h_slice[0] = init_val;
            }
            for h in h_slice.iter_mut() {
                *h = physics::propagate_head(*h, v_seg);
            }
            heads_arr.h0 = h_slice[0];
            heads_arr.h1 = h_slice[1];
            heads_arr.h2 = h_slice[2];
            heads_arr.h3 = h_slice[3];
            heads_arr.h4 = h_slice[4];
            heads_arr.h5 = h_slice[5];
            heads_arr.h6 = h_slice[6];
            heads_arr.h7 = h_slice[7];
        };

        apply_injections_and_propagate(&mut expected_heads[0], 1); // Axon 0
        apply_injections_and_propagate(&mut expected_heads[1], 2); // Axon 1
        apply_injections_and_propagate(&mut expected_heads[2], 3); // Axon 2
        apply_injections_and_propagate(&mut expected_heads[3], 0); // Axon 3 (no injections)

        let mut expected_axons_blob = vec![0u8; axons_size];
        expected_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));
        expected_axons_blob[16..16 + heads_bytes.len()]
            .copy_from_slice(bytemuck::cast_slice(&expected_heads));

        assert_eq!(snap_axons, expected_axons_blob);

        // Test no-input case matches simple propagation
        let upload2 = compute_api::ShardUpload {
            state_blob: &vec![0u8; state_size],
            axons_blob: &initial_axons_blob,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload2).unwrap();

        backend
            .inject_and_propagate_axons_tick_for_test(handle, v_seg, 95, 10, None, None)
            .unwrap();

        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        let mut expected_heads2 = heads;
        for eh in expected_heads2.iter_mut() {
            apply_injections_and_propagate(eh, 0);
        }
        let mut expected_axons_blob2 = vec![0u8; axons_size];
        expected_axons_blob2[..16].copy_from_slice(bytemuck::bytes_of(&header));
        expected_axons_blob2[16..16 + heads_bytes.len()]
            .copy_from_slice(bytemuck::cast_slice(&expected_heads2));

        assert_eq!(snap_axons, expected_axons_blob2);

        // 1. Test short input_bitmask fails with InvalidBatch
        let short_mask = vec![];
        let res_short = backend.inject_and_propagate_axons_tick_for_test(
            handle,
            v_seg,
            95,
            10,
            Some(&short_mask),
            None,
        );
        assert!(matches!(res_short, Err(ComputeApiError::InvalidBatch)));

        // 2. Test virtual range near u32::MAX does not overflow
        let mask_ok = vec![0; 1];
        let res_u32_max = backend.inject_and_propagate_axons_tick_for_test(
            handle,
            v_seg,
            u32::MAX - 5,
            10,
            Some(&mask_ok),
            None,
        );
        assert!(res_u32_max.is_ok());

        // 3. Test 9+ duplicate incoming spikes in one axon
        let upload3 = compute_api::ShardUpload {
            state_blob: &vec![0u8; state_size],
            axons_blob: &initial_axons_blob,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload3).unwrap();

        let duplicate_spikes = vec![1; 10]; // 10 spikes for local axon 1
        backend
            .inject_and_propagate_axons_tick_for_test(
                handle,
                v_seg,
                95,
                10,
                None,
                Some(&duplicate_spikes),
            )
            .unwrap();

        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        let mut expected_heads3 = heads;
        apply_injections_and_propagate(&mut expected_heads3[0], 0);
        apply_injections_and_propagate(&mut expected_heads3[1], 8); // Clamped to 8
        apply_injections_and_propagate(&mut expected_heads3[2], 0);
        apply_injections_and_propagate(&mut expected_heads3[3], 0);

        let mut expected_axons_blob3 = vec![0u8; axons_size];
        expected_axons_blob3[..16].copy_from_slice(bytemuck::bytes_of(&header));
        expected_axons_blob3[16..16 + heads_bytes.len()]
            .copy_from_slice(bytemuck::cast_slice(&expected_heads3));

        assert_eq!(snap_axons, expected_axons_blob3);

        // 4. Test virtual range near u32::MAX with actual injections (overflow check)
        let spec_overflow = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 2,
            total_ghosts: 0,
            virtual_offset: u32::MAX - 1,
        };
        let handle_overflow = backend.alloc_shard(spec_overflow).unwrap();
        let state_size_ov = layout::calculate_state_blob_size(spec_overflow.padded_n as usize);
        let axons_size_ov =
            compute_api::validation::expected_axons_blob_size(spec_overflow.total_axons).unwrap();

        let header_ov = layout::AxonsFileHeader::new(spec_overflow.total_axons);
        let mut initial_axons_blob_ov = vec![0u8; axons_size_ov];
        initial_axons_blob_ov[..16].copy_from_slice(bytemuck::bytes_of(&header_ov));

        let mut heads_ov = [
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
        ];
        heads_ov[0].h0 = 1000;
        heads_ov[1].h0 = 2000;

        let heads_bytes_ov = bytemuck::cast_slice(&heads_ov);
        initial_axons_blob_ov[16..16 + heads_bytes_ov.len()].copy_from_slice(heads_bytes_ov);

        let upload_ov = compute_api::ShardUpload {
            state_blob: &vec![0u8; state_size_ov],
            axons_blob: &initial_axons_blob_ov,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle_overflow, upload_ov).unwrap();

        // cmd_virtual_offset = u32::MAX - 1, num_virtual_axons = 2, bitmask = [3] (activates both)
        let mask_overflow = vec![3];
        backend
            .inject_and_propagate_axons_tick_for_test(
                handle_overflow,
                v_seg,
                u32::MAX - 1,
                2,
                Some(&mask_overflow),
                None,
            )
            .unwrap();

        let mut snap_state_ov = vec![0u8; state_size_ov];
        let mut snap_axons_ov = vec![0u8; axons_size_ov];
        backend
            .debug_snapshot(
                handle_overflow,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state_ov,
                    axons_blob: &mut snap_axons_ov,
                },
            )
            .unwrap();

        // Both axons should have 1 injection (virtual) and then propagated
        let mut expected_heads_ov = heads_ov;
        apply_injections_and_propagate(&mut expected_heads_ov[0], 1);
        apply_injections_and_propagate(&mut expected_heads_ov[1], 1);

        let mut expected_axons_blob_ov = vec![0u8; axons_size_ov];
        expected_axons_blob_ov[..16].copy_from_slice(bytemuck::bytes_of(&header_ov));
        expected_axons_blob_ov[16..16 + heads_bytes_ov.len()]
            .copy_from_slice(bytemuck::cast_slice(&expected_heads_ov));

        assert_eq!(snap_axons_ov, expected_axons_blob_ov);
        backend.free_shard(handle_overflow).unwrap();

        backend.free_shard(handle).unwrap();
    }

    #[test]
    #[cfg(feature = "native")]
    #[allow(clippy::needless_range_loop, clippy::identity_op, clippy::erasing_op)]
    fn test_cuda_native_compute_input_current_probe() {
        if !is_gpu_available() {
            return;
        }
        let _lock = GPU_TEST_LOCK.lock().unwrap();
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 3,
            total_ghosts: 0,
            virtual_offset: 100,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_size =
            compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

        // 1. Set up state blob
        let mut test_state = vec![0u8; state_size];
        let offsets = layout::compute_state_offsets(spec.padded_n as usize);

        let mut targets =
            vec![types::PackedTarget::NONE; layout::MAX_DENDRITES * spec.padded_n as usize];
        let mut weights = vec![0i32; layout::MAX_DENDRITES * spec.padded_n as usize];

        // Soma 0
        // Dendrite 0: target axon 1, segment 10. weight = 1600 << 16
        targets[0 * spec.padded_n as usize + 0] = types::PackedTarget::pack(1, 10);
        weights[0 * spec.padded_n as usize + 0] = 1600 << 16;
        // Dendrite 1: target = NONE, weight = 500 << 16
        targets[1 * spec.padded_n as usize + 0] = types::PackedTarget::NONE;
        weights[1 * spec.padded_n as usize + 0] = 500 << 16;
        // Dendrite 2: target axon_q = 0x00FF_FFFF (corrupt/reserved), weight = 999 << 16 (ignored)
        targets[2 * spec.padded_n as usize + 0] = types::PackedTarget(0x0AFF_FFFF);
        weights[2 * spec.padded_n as usize + 0] = 999 << 16;
        // Dendrite 3: target axon 0, segment 0. weight = -800 << 16
        targets[3 * spec.padded_n as usize + 0] = types::PackedTarget::pack(0, 0);
        weights[3 * spec.padded_n as usize + 0] = -800 << 16;

        // Soma 1
        // Dendrite 0: target = TOMBSTONE (inactive)
        targets[0 * spec.padded_n as usize + 1] = types::PackedTarget::TOMBSTONE;
        weights[0 * spec.padded_n as usize + 1] = 500 << 16;
        // Dendrite 1: target axon 2, segment 50 (miss). weight = 3200 << 16
        targets[1 * spec.padded_n as usize + 1] = types::PackedTarget::pack(2, 50);
        weights[1 * spec.padded_n as usize + 1] = 3200 << 16;

        // Soma 2
        // Dendrite 0: target axon 1, segment 0 (miss). weight = i32::MAX
        targets[0 * spec.padded_n as usize + 2] = types::PackedTarget::pack(1, 0);
        weights[0 * spec.padded_n as usize + 2] = i32::MAX;
        // Dendrite 1: target axon 1, segment 0 (miss). weight = i32::MAX
        targets[1 * spec.padded_n as usize + 2] = types::PackedTarget::pack(1, 0);
        weights[1 * spec.padded_n as usize + 2] = i32::MAX;
        // Dendrite 2: target axon 1, segment 10 (hit). weight = i32::MIN
        targets[2 * spec.padded_n as usize + 2] = types::PackedTarget::pack(1, 10);
        weights[2 * spec.padded_n as usize + 2] = i32::MIN;
        // Dendrite 3: target axon 1, segment 10 (hit). weight = i32::MIN
        targets[3 * spec.padded_n as usize + 2] = types::PackedTarget::pack(1, 10);
        weights[3 * spec.padded_n as usize + 2] = i32::MIN;

        // Copy targets and weights to state blob
        let targets_bytes = bytemuck::cast_slice(&targets);
        test_state[offsets.off_targets..offsets.off_targets + targets_bytes.len()]
            .copy_from_slice(targets_bytes);

        let weights_bytes = bytemuck::cast_slice(&weights);
        test_state[offsets.off_weights..offsets.off_weights + weights_bytes.len()]
            .copy_from_slice(weights_bytes);

        // 2. Set up axon heads
        let header = layout::AxonsFileHeader::new(spec.total_axons);
        let mut test_axons_blob = vec![0u8; axons_size];
        test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

        let mut heads = [
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
        ];
        // Axon 0
        heads[0].h0 = 0; // segment 0 (active)
                         // Axon 1
        heads[1].h0 = 10; // segment 10 (active)
                          // Axon 2
        heads[2].h0 = 20; // segment 20 (active)

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
            signal_propagation_length: 5, // prop len = 5
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

        // 3. Test compute input current probe
        // Output buffer size is larger than padded_n to test that the tail is not overwritten
        let mut out_i_in = vec![0i32; spec.padded_n as usize + 10];
        for i in spec.padded_n as usize..spec.padded_n as usize + 10 {
            out_i_in[i] = 999;
        }

        let propagation_length = 5;
        backend
            .compute_input_current_probe_for_test(handle, propagation_length, &mut out_i_in)
            .unwrap();

        // Soma 0: Dendrite 0 (hit, weight = 1600 >> 16 -> charge 1600)
        //         Dendrite 3 (hit, weight = -800 >> 16 -> charge -800)
        //         Expected = 800
        assert_eq!(out_i_in[0], 800);

        // Soma 1: Dendrite 1 (miss, segment 50 vs head 20 -> d = 20 - 50 = 0xFFFFFFE2 >= 5 -> miss)
        //         Expected = 0
        assert_eq!(out_i_in[1], 0);

        // Soma 2: Dendrite 2 (hit, weight = i32::MIN -> charge = -32768)
        //         Dendrite 3 (hit, weight = i32::MIN -> charge = -32768)
        //         Expected = -65536 (computed via i32::wrapping_add on GPU)
        let expected_soma2 = (i32::MIN >> 16).wrapping_add(i32::MIN >> 16);
        assert_eq!(out_i_in[2], expected_soma2);

        // Remaining somas (3..64): expected = 0
        for i in 3..spec.padded_n as usize {
            assert_eq!(out_i_in[i], 0, "Soma {} mismatch", i);
        }

        // Check that the tail (from padded_n onwards) remains untouched
        for i in spec.padded_n as usize..spec.padded_n as usize + 10 {
            assert_eq!(out_i_in[i], 999, "Tail element {} was overwritten", i);
        }

        backend.free_shard(handle).unwrap();
    }

    #[test]
    #[cfg(feature = "native")]
    #[allow(clippy::bool_assert_comparison)]
    fn test_cuda_native_apply_glif_membrane_probe() {
        if !is_gpu_available() {
            return;
        }
        let _lock = GPU_TEST_LOCK.lock().unwrap();
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 1,
            total_ghosts: 0,
            virtual_offset: 100,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_size =
            compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

        // 1. Set up state blob
        let mut test_state = vec![0u8; state_size];
        let offsets = layout::compute_state_offsets(spec.padded_n as usize);

        let mut voltages = vec![0i32; spec.padded_n as usize];
        let mut flags = vec![0u8; spec.padded_n as usize];
        let mut thresh_offsets = vec![0i32; spec.padded_n as usize];
        let mut timers = vec![0u8; spec.padded_n as usize];

        // Variant parameters:
        // Variant 0: normal parameters, leak_shift = 4
        // Variant 1: adaptive leak parameters, leak_shift = 8
        let variant_0 = layout::VariantParameters {
            threshold: -50_000,
            rest_potential: -70_000,
            leak_shift: 4,
            homeostasis_penalty: 1000,
            spontaneous_firing_period_ticks: 0,
            initial_synapse_weight: 0,
            gsop_potentiation: 0,
            gsop_depression: 0,
            homeostasis_decay: 100,
            refractory_period: 5,
            synapse_refractory_period: 0,
            signal_propagation_length: 5,
            is_inhibitory: 0,
            inertia_curve: [0; 8],
            ahp_amplitude: 10_000,
            _pad1: [0; 6],
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
            _leak_pad: [0; 3],
            d1_affinity: 0,
            d2_affinity: 0,
            heartbeat_m: 0,
        };

        let mut variant_1 = variant_0;
        variant_1.homeostasis_decay = 256;
        variant_1.leak_shift = 16;
        variant_1.adaptive_leak_min_shift = 2;
        variant_1.adaptive_leak_gain = 4;
        variant_1.adaptive_mode = 1;

        let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
        variant_table[1] = variant_1;

        // Soma 0: normal no-spike voltage update (Variant 0)
        // Starting voltage: -60_000, type = 0
        voltages[0] = -60_000;
        flags[0] = types::SomaFlags::new(false, 0, 0).0;
        thresh_offsets[0] = 0;
        timers[0] = 0;

        // Soma 1: GLIF spike (Variant 0)
        // Starting voltage: -49_000, type = 0
        voltages[1] = -49_000;
        flags[1] = types::SomaFlags::new(false, 0, 0).0;
        thresh_offsets[1] = 0;
        timers[1] = 0;

        // Soma 2: refractory timer (Variant 0)
        // Starting voltage: -80_000, type = 0
        voltages[2] = -80_000;
        flags[2] = types::SomaFlags::new(false, 0, 0).0;
        thresh_offsets[2] = 1000;
        timers[2] = 3;

        // Soma 3: burst_count already 7 (Variant 0)
        // Starting voltage: -49_000, type = 0, burst = 7
        voltages[3] = -49_000;
        flags[3] = types::SomaFlags::new(false, 7, 0).0;
        thresh_offsets[3] = 0;
        timers[3] = 0;

        // Soma 4: adaptive leak (Variant 1)
        // Starting voltage: -60_000, type = 1 (Variant 1)
        voltages[4] = -60_000;
        flags[4] = types::SomaFlags::new(false, 0, 1).0;
        thresh_offsets[4] = 512;
        timers[4] = 0;

        // Write to state blob
        test_state[offsets.off_voltage..offsets.off_voltage + voltages.len() * 4]
            .copy_from_slice(bytemuck::cast_slice(&voltages));
        test_state[offsets.off_flags..offsets.off_flags + flags.len()].copy_from_slice(&flags);
        test_state[offsets.off_thresh..offsets.off_thresh + thresh_offsets.len() * 4]
            .copy_from_slice(bytemuck::cast_slice(&thresh_offsets));
        test_state[offsets.off_timers..offsets.off_timers + timers.len()].copy_from_slice(&timers);

        let header = layout::AxonsFileHeader::new(spec.total_axons);
        let mut test_axons_blob = vec![0u8; axons_size];
        test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons_blob,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        // Input currents
        let mut i_in = vec![0i32; spec.padded_n as usize];
        i_in[0] = 2000;
        i_in[1] = 1000;
        i_in[2] = 5000; // ignored
        i_in[3] = 1000;
        i_in[4] = 2000;

        // Verify that short i_in returns InvalidBatch
        let res_short = backend.apply_glif_membrane_probe_for_test(handle, &i_in[..5]);
        assert!(matches!(res_short, Err(ComputeApiError::InvalidBatch)));

        // Run membrane probe update
        backend
            .apply_glif_membrane_probe_for_test(handle, &i_in)
            .unwrap();

        // Download snapshot
        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        let snap_voltages: &[i32] = bytemuck::cast_slice(
            &snap_state[offsets.off_voltage..offsets.off_voltage + spec.padded_n as usize * 4],
        );
        let snap_flags: &[u8] =
            &snap_state[offsets.off_flags..offsets.off_flags + spec.padded_n as usize];
        let snap_thresh: &[i32] = bytemuck::cast_slice(
            &snap_state[offsets.off_thresh..offsets.off_thresh + spec.padded_n as usize * 4],
        );
        let snap_timers: &[u8] =
            &snap_state[offsets.off_timers..offsets.off_timers + spec.padded_n as usize];

        let mut expected_voltages = voltages.clone();
        let mut expected_flags = flags.clone();
        let mut expected_thresh = thresh_offsets.clone();
        let mut expected_timers = timers.clone();

        for i in 0..5 {
            let variant_idx = types::SomaFlags(flags[i]).type_id() as usize;
            let var = variant_table[variant_idx];

            let timer = timers[i];
            let thresh_offset = thresh_offsets[i];
            let voltage = voltages[i];
            let i_in_val = i_in[i];

            let decayed_offset =
                physics::homeostasis_decay(thresh_offset, var.homeostasis_decay as i32);

            if timer > 0 {
                expected_timers[i] = timer - 1;
                expected_thresh[i] = decayed_offset;
                expected_voltages[i] = voltage;
                expected_flags[i] = types::SomaFlags::new(
                    false,
                    types::SomaFlags(flags[i]).burst_count(),
                    variant_idx as u8,
                )
                .0;
            } else {
                let v_new = physics::update_glif_voltage(
                    voltage,
                    i_in_val,
                    var.rest_potential,
                    decayed_offset,
                    var.leak_shift as i32,
                    var.adaptive_leak_gain as i32,
                    var.adaptive_leak_min_shift as i32,
                    var.adaptive_mode as i32,
                );

                let is_glif = physics::is_glif_spike(v_new, var.threshold, decayed_offset);

                if is_glif {
                    expected_voltages[i] =
                        var.rest_potential.wrapping_sub(var.ahp_amplitude as i32);
                    expected_timers[i] = var.refractory_period;
                    expected_thresh[i] = decayed_offset.wrapping_add(var.homeostasis_penalty);
                    let new_burst = (types::SomaFlags(flags[i]).burst_count() + 1).min(7);
                    expected_flags[i] = types::SomaFlags::new(true, new_burst, variant_idx as u8).0;
                } else {
                    expected_voltages[i] = v_new;
                    expected_timers[i] = 0;
                    expected_thresh[i] = decayed_offset;
                    expected_flags[i] = types::SomaFlags::new(
                        false,
                        types::SomaFlags(flags[i]).burst_count(),
                        variant_idx as u8,
                    )
                    .0;
                }
            }
        }

        // Hardcoded assertions derived from CPU expected values
        assert_eq!(expected_voltages[0], -58625);
        assert_eq!(expected_voltages[1], -80000);
        assert_eq!(expected_voltages[2], -80000);
        assert_eq!(expected_voltages[3], -80000);
        assert_eq!(expected_voltages[4], -58002);

        for i in 0..5 {
            assert_eq!(
                snap_voltages[i], expected_voltages[i],
                "Soma {} voltage mismatch",
                i
            );
            assert_eq!(
                snap_flags[i], expected_flags[i],
                "Soma {} flags mismatch",
                i
            );
            assert_eq!(
                snap_thresh[i], expected_thresh[i],
                "Soma {} thresh mismatch",
                i
            );
            assert_eq!(
                snap_timers[i], expected_timers[i],
                "Soma {} timer mismatch",
                i
            );
        }

        backend.free_shard(handle).unwrap();
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_cuda_native_multi_shard_variant_table_isolation() {
        if !is_gpu_available() {
            return;
        }
        let _lock = GPU_TEST_LOCK.lock().unwrap();
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

        let spec_a = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 1,
            total_ghosts: 0,
            virtual_offset: 100,
        };
        let handle_a = backend.alloc_shard(spec_a).unwrap();

        let spec_b = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 1,
            total_ghosts: 0,
            virtual_offset: 200,
        };
        let handle_b = backend.alloc_shard(spec_b).unwrap();

        let state_size = layout::calculate_state_blob_size(64);
        let axons_size = compute_api::validation::expected_axons_blob_size(1).unwrap();

        // 1. Shard A upload: variant A (leak_shift = 4)
        let mut test_state_a = vec![0u8; state_size];
        let offsets = layout::compute_state_offsets(64);
        let mut voltages_a = vec![0i32; 64];
        voltages_a[0] = -60_000;
        test_state_a[offsets.off_voltage..offsets.off_voltage + 256]
            .copy_from_slice(bytemuck::cast_slice(&voltages_a));

        let variant_a = layout::VariantParameters {
            threshold: -50_000,
            rest_potential: -70_000,
            leak_shift: 4, // leak_shift = 4
            homeostasis_penalty: 1000,
            spontaneous_firing_period_ticks: 0,
            initial_synapse_weight: 0,
            gsop_potentiation: 0,
            gsop_depression: 0,
            homeostasis_decay: 100,
            refractory_period: 5,
            synapse_refractory_period: 0,
            signal_propagation_length: 5,
            is_inhibitory: 0,
            inertia_curve: [0; 8],
            ahp_amplitude: 10_000,
            _pad1: [0; 6],
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
            _leak_pad: [0; 3],
            d1_affinity: 0,
            d2_affinity: 0,
            heartbeat_m: 0,
        };
        let variant_table_a = [variant_a; layout::VARIANT_LUT_LEN];

        let header = layout::AxonsFileHeader::new(1);
        let mut test_axons = vec![0u8; axons_size];
        test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));

        backend
            .upload_shard(
                handle_a,
                compute_api::ShardUpload {
                    state_blob: &test_state_a,
                    axons_blob: &test_axons,
                    variant_table: &variant_table_a,
                },
            )
            .unwrap();

        // 2. Shard B upload: variant B (leak_shift = 8)
        let test_state_b = vec![0u8; state_size];
        let mut variant_b = variant_a;
        variant_b.leak_shift = 8; // leak_shift = 8
        let variant_table_b = [variant_b; layout::VARIANT_LUT_LEN];

        backend
            .upload_shard(
                handle_b,
                compute_api::ShardUpload {
                    state_blob: &test_state_b,
                    axons_blob: &test_axons,
                    variant_table: &variant_table_b,
                },
            )
            .unwrap();

        // 3. Call run_current_glif_tick_probe_for_test on Shard A
        backend
            .run_current_glif_tick_probe_for_test(handle_a, 5)
            .unwrap();

        // 4. Download Shard A snapshot and check it matches variant A (leak_shift = 4)
        let mut snap_state_a = vec![0u8; state_size];
        let mut snap_axons_a = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle_a,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state_a,
                    axons_blob: &mut snap_axons_a,
                },
            )
            .unwrap();

        let snap_voltages_a: &[i32] =
            bytemuck::cast_slice(&snap_state_a[offsets.off_voltage..offsets.off_voltage + 256]);

        // Under variant A (leak_shift = 4): expected = -60625
        assert_eq!(snap_voltages_a[0], -60625);

        backend.free_shard(handle_a).unwrap();
        backend.free_shard(handle_b).unwrap();
    }

    #[test]
    #[cfg(feature = "native")]
    #[allow(clippy::needless_range_loop, clippy::identity_op, clippy::erasing_op)]
    fn test_cuda_native_current_glif_tick_probe() {
        if !is_gpu_available() {
            return;
        }
        let _lock = GPU_TEST_LOCK.lock().unwrap();
        let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

        let spec = compute_api::ShardAllocSpec {
            padded_n: 64,
            total_axons: 3,
            total_ghosts: 0,
            virtual_offset: 100,
        };
        let handle = backend.alloc_shard(spec).unwrap();

        let state_size = layout::calculate_state_blob_size(64);
        let axons_size = compute_api::validation::expected_axons_blob_size(3).unwrap();

        let mut test_state = vec![0u8; state_size];
        let offsets = layout::compute_state_offsets(64);

        // Synapses targets & weights:
        let mut dendrite_targets = vec![0u32; 64 * 128];
        let mut dendrite_weights = vec![0i32; 64 * 128];

        // Soma 0: target local axon 0 segment 5, positive weight (charge 1600)
        dendrite_targets[0 * 64 + 0] = types::PackedTarget::pack(0, 5).0;
        dendrite_weights[0 * 64 + 0] = 1600 << 16;

        // Soma 1: target local axon 1 segment 10, negative weight (charge -800)
        dendrite_targets[0 * 64 + 1] = types::PackedTarget::pack(1, 10).0;
        dendrite_weights[0 * 64 + 1] = -800 << 16;

        // Soma 2: target local axon 2 segment 20, positive weight (charge 500)
        dendrite_targets[0 * 64 + 2] = types::PackedTarget::pack(2, 20).0;
        dendrite_weights[0 * 64 + 2] = 500 << 16;

        // Copy dendrite planes to state blob
        test_state[offsets.off_targets..offsets.off_targets + dendrite_targets.len() * 4]
            .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
        test_state[offsets.off_weights..offsets.off_weights + dendrite_weights.len() * 4]
            .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

        // Initialize soma planes
        let mut voltages = vec![0i32; 64];
        let mut flags = vec![0u8; 64];
        let mut thresh_offsets = vec![0i32; 64];
        let mut timers = vec![0u8; 64];

        // Soma 0: normal voltage trigger spike
        voltages[0] = -49_000;
        flags[0] = types::SomaFlags::new(false, 0, 0).0;
        thresh_offsets[0] = 0;
        timers[0] = 0;

        // Soma 1: normal negative update, no spike
        voltages[1] = -60_000;
        flags[1] = types::SomaFlags::new(false, 0, 0).0;
        thresh_offsets[1] = 0;
        timers[1] = 0;

        // Soma 2: refractory period, decrement timer, decay threshold
        voltages[2] = -80_000;
        flags[2] = types::SomaFlags::new(false, 0, 0).0;
        thresh_offsets[2] = 1000;
        timers[2] = 3;

        // Soma 3: check variant 2 and type_id preservation
        voltages[3] = -60_000;
        flags[3] = types::SomaFlags::new(false, 0, 2).0;
        thresh_offsets[3] = 0;
        timers[3] = 0;

        // Copy soma planes to state blob
        test_state[offsets.off_voltage..offsets.off_voltage + voltages.len() * 4]
            .copy_from_slice(bytemuck::cast_slice(&voltages));
        test_state[offsets.off_flags..offsets.off_flags + flags.len()].copy_from_slice(&flags);
        test_state[offsets.off_thresh..offsets.off_thresh + thresh_offsets.len() * 4]
            .copy_from_slice(bytemuck::cast_slice(&thresh_offsets));
        test_state[offsets.off_timers..offsets.off_timers + timers.len()].copy_from_slice(&timers);

        // Axon heads Setup:
        let header = layout::AxonsFileHeader::new(3);
        let mut test_axons_blob = vec![0u8; axons_size];
        test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

        let mut heads = [
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
            layout::BurstHeads8::empty(types::AXON_SENTINEL),
        ];
        heads[0].h0 = 5;
        heads[1].h0 = 10;
        heads[2].h0 = 20;
        test_axons_blob[16..16 + 3 * 32].copy_from_slice(bytemuck::cast_slice(&heads));

        // Variant parameters:
        let variant_0 = layout::VariantParameters {
            threshold: -50_000,
            rest_potential: -70_000,
            leak_shift: 4,
            homeostasis_penalty: 1000,
            spontaneous_firing_period_ticks: 0,
            initial_synapse_weight: 0,
            gsop_potentiation: 0,
            gsop_depression: 0,
            homeostasis_decay: 100,
            refractory_period: 5,
            synapse_refractory_period: 0,
            signal_propagation_length: 5,
            is_inhibitory: 0,
            inertia_curve: [0; 8],
            ahp_amplitude: 10_000,
            _pad1: [0; 6],
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
            _leak_pad: [0; 3],
            d1_affinity: 0,
            d2_affinity: 0,
            heartbeat_m: 0,
        };

        let mut variant_2 = variant_0;
        variant_2.rest_potential = -65_000;
        variant_2.leak_shift = 5;

        let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
        variant_table[2] = variant_2;

        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons_blob,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        // 3. Run tick probe
        let propagation_length = 5;
        backend
            .run_current_glif_tick_probe_for_test(handle, propagation_length)
            .unwrap();

        // 4. Download snapshot
        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        // Check that axons snapshot is byte-exact compared to original axons blob
        assert_eq!(snap_axons, test_axons_blob);

        let snap_voltages: &[i32] =
            bytemuck::cast_slice(&snap_state[offsets.off_voltage..offsets.off_voltage + 256]);
        let snap_flags: &[u8] = &snap_state[offsets.off_flags..offsets.off_flags + 64];
        let snap_thresh: &[i32] =
            bytemuck::cast_slice(&snap_state[offsets.off_thresh..offsets.off_thresh + 256]);
        let snap_timers: &[u8] = &snap_state[offsets.off_timers..offsets.off_timers + 64];

        // Compute expectations on CPU
        let mut expected_voltages = voltages.clone();
        let mut expected_flags = flags.clone();
        let mut expected_thresh = thresh_offsets.clone();
        let mut expected_timers = timers.clone();
        let mut expected_i_in = vec![0i32; 64];

        for s in 0..64 {
            let mut charge_sum = 0i32;
            for d in 0..128 {
                let d_idx = d * 64 + s;
                let target_raw = dendrite_targets[d_idx];
                if target_raw != 0 {
                    let target = types::PackedTarget(target_raw);
                    if let Some((axon_id, segment_index)) = target.unpack() {
                        let local_axon = axon_id as usize;
                        if local_axon < spec.total_axons as usize {
                            let mut axon_heads_array = [types::AXON_SENTINEL; 8];
                            let h = heads[local_axon];
                            axon_heads_array[0] = h.h0;
                            axon_heads_array[1] = h.h1;
                            axon_heads_array[2] = h.h2;
                            axon_heads_array[3] = h.h3;
                            axon_heads_array[4] = h.h4;
                            axon_heads_array[5] = h.h5;
                            axon_heads_array[6] = h.h6;
                            axon_heads_array[7] = h.h7;

                            if physics::active_tail_hit(
                                &axon_heads_array,
                                segment_index,
                                propagation_length,
                            ) {
                                let weight = dendrite_weights[d_idx];
                                charge_sum =
                                    charge_sum.wrapping_add(physics::weight_to_charge(weight));
                            }
                        }
                    }
                }
            }
            expected_i_in[s] = charge_sum;
        }

        for i in 0..4 {
            let variant_idx = types::SomaFlags(flags[i]).type_id() as usize;
            let var = variant_table[variant_idx];

            let timer = timers[i];
            let thresh_offset = thresh_offsets[i];
            let voltage = voltages[i];
            let i_in_val = expected_i_in[i];

            let decayed_offset =
                physics::homeostasis_decay(thresh_offset, var.homeostasis_decay as i32);

            if timer > 0 {
                expected_timers[i] = timer - 1;
                expected_thresh[i] = decayed_offset;
                expected_voltages[i] = voltage;
                expected_flags[i] = types::SomaFlags::new(
                    false,
                    types::SomaFlags(flags[i]).burst_count(),
                    variant_idx as u8,
                )
                .0;
            } else {
                let v_new = physics::update_glif_voltage(
                    voltage,
                    i_in_val,
                    var.rest_potential,
                    decayed_offset,
                    var.leak_shift as i32,
                    var.adaptive_leak_gain as i32,
                    var.adaptive_leak_min_shift as i32,
                    var.adaptive_mode as i32,
                );

                let is_glif = physics::is_glif_spike(v_new, var.threshold, decayed_offset);

                if is_glif {
                    expected_voltages[i] =
                        var.rest_potential.wrapping_sub(var.ahp_amplitude as i32);
                    expected_timers[i] = var.refractory_period;
                    expected_thresh[i] = decayed_offset.wrapping_add(var.homeostasis_penalty);
                    let new_burst = (types::SomaFlags(flags[i]).burst_count() + 1).min(7);
                    expected_flags[i] = types::SomaFlags::new(true, new_burst, variant_idx as u8).0;
                } else {
                    expected_voltages[i] = v_new;
                    expected_timers[i] = 0;
                    expected_thresh[i] = decayed_offset;
                    expected_flags[i] = types::SomaFlags::new(
                        false,
                        types::SomaFlags(flags[i]).burst_count(),
                        variant_idx as u8,
                    )
                    .0;
                }
            }
        }

        // Assert expected properties
        assert_eq!(expected_voltages[0], -80_000);
        assert_eq!(expected_voltages[1], -61_425);
        assert_eq!(expected_voltages[2], -80_000);
        assert_eq!(expected_timers[2], 2);
        assert_eq!(expected_thresh[2], 900);
        assert_eq!(types::SomaFlags(expected_flags[3]).type_id(), 2);

        for i in 0..4 {
            assert_eq!(
                snap_voltages[i], expected_voltages[i],
                "Soma {} voltage mismatch",
                i
            );
            assert_eq!(
                snap_flags[i], expected_flags[i],
                "Soma {} flags mismatch",
                i
            );
            assert_eq!(
                snap_thresh[i], expected_thresh[i],
                "Soma {} thresh mismatch",
                i
            );
            assert_eq!(
                snap_timers[i], expected_timers[i],
                "Soma {} timer mismatch",
                i
            );
        }

        backend.free_shard(handle).unwrap();
    }
}
