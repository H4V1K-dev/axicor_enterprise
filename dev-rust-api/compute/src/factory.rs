use compute_api::{GpuBackend, ComputeApiError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Cuda,
    Hip,
    Cpu,
}

/// Detects available compute backends on the host system in order of preference.
pub fn detect_available_backends() -> Vec<BackendType> {
    let mut backends = Vec::new();
    // For now, Cuda and Hip backends are not available.
    // Return Cpu backend if compiled with the cpu feature flag enabled.
    #[cfg(feature = "cpu")]
    {
        backends.push(BackendType::Cpu);
    }
    backends
}

/// Instantiates a dynamic `GpuBackend` instance for the given platform type and device ID.
pub fn instantiate_backend(
    backend: BackendType,
    device_id: Option<i32>,
) -> Result<Box<dyn GpuBackend>, ComputeApiError> {
    // E-069: If an invalid device ID is passed (e.g. out of bounds), return DeviceLost
    if let Some(id) = device_id {
        if id < 0 || id > 0 {
            return Err(ComputeApiError::DeviceLost);
        }
    }

    match backend {
        BackendType::Cpu => {
            #[cfg(feature = "cpu")]
            {
                let cpu_backend = compute_cpu::CpuBackend::new()?;
                Ok(Box::new(cpu_backend))
            }
            #[cfg(not(feature = "cpu"))]
            {
                Err(ComputeApiError::DeviceLost)
            }
        }
        BackendType::Cuda => {
            // CUDA backend is not implemented yet
            Err(ComputeApiError::DeviceLost)
        }
        BackendType::Hip => {
            // HIP backend is not implemented yet
            Err(ComputeApiError::DeviceLost)
        }
    }
}
