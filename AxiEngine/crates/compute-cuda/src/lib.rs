//! Scaffold implementation of CudaBackend for AxiEngine Layer 3.

use std::marker::PhantomData;
use std::rc::Rc;

use compute_api::{BackendCapabilities, BackendKind, ComputeApiError, ComputeBackend};

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
    // Statically prevent Send and Sync
    _marker: PhantomData<Rc<()>>,
}

impl CudaBackend {
    /// Creates a new instance of the CUDA compute backend.
    ///
    /// # Errors
    /// Returns `ComputeApiError::UnsupportedBackend` in Stage 1A when native drivers/features are absent.
    pub fn new(config: CudaBackendConfig) -> Result<Self, ComputeApiError> {
        let _ = config;
        #[cfg(not(feature = "native"))]
        {
            Err(ComputeApiError::UnsupportedBackend)
        }
        #[cfg(feature = "native")]
        {
            // Native provider implementation is deferred to Stage 1B.
            Err(ComputeApiError::BackendNotInitialized)
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
        _spec: compute_api::ShardAllocSpec,
    ) -> Result<compute_api::VramHandle, ComputeApiError> {
        Err(ComputeApiError::UnsupportedBackend)
    }

    fn upload_shard(
        &mut self,
        _handle: compute_api::VramHandle,
        _upload: compute_api::ShardUpload<'_>,
    ) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedBackend)
    }

    fn run_day_batch(
        &mut self,
        _handle: compute_api::VramHandle,
        _cmd: compute_api::DayBatchCmd<'_>,
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        Err(ComputeApiError::UnsupportedBackend)
    }

    fn free_shard(&mut self, _handle: compute_api::VramHandle) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedBackend)
    }

    fn debug_snapshot(
        &mut self,
        _handle: compute_api::VramHandle,
        _snapshot: compute_api::ShardSnapshotMut<'_>,
    ) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedBackend)
    }

    fn teardown(&mut self) -> Result<(), ComputeApiError> {
        Ok(())
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
}
