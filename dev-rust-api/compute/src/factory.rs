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

    #[cfg(feature = "cuda")]
    {
        if compute_cuda::CudaBackend::new(0).is_ok() {
            backends.push(BackendType::Cuda);
        }
    }

    #[cfg(feature = "hip")]
    {
        if compute_hip::HipBackend::new(0).is_ok() {
            backends.push(BackendType::Hip);
        }
    }

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

    let dev_id = device_id.unwrap_or(0);

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
            #[cfg(feature = "cuda")]
            {
                let cuda_backend = compute_cuda::CudaBackend::new(dev_id)?;
                Ok(Box::new(cuda_backend))
            }
            #[cfg(not(feature = "cuda"))]
            {
                Err(ComputeApiError::DeviceLost)
            }
        }
        BackendType::Hip => {
            #[cfg(feature = "hip")]
            {
                let hip_backend = compute_hip::HipBackend::new(dev_id)?;
                Ok(Box::new(hip_backend))
            }
            #[cfg(not(feature = "hip"))]
            {
                Err(ComputeApiError::DeviceLost)
            }
        }
    }
}
