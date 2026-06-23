pub mod factory;
pub mod engine;

pub use factory::*;
pub use engine::*;

#[cfg(test)]
mod tests {
    use super::*;
    use compute_api::{ShardLayout, ComputeApiError};

    #[test]
    fn test_backend_auto_selection() {
        // INV-COMPUTE-002: auto-selection validation
        let backends = detect_available_backends();
        
        #[cfg(feature = "cpu")]
        {
            assert!(backends.contains(&BackendType::Cpu));
        }

        #[cfg(feature = "cuda")]
        {
            assert_eq!(backends[0], BackendType::Cuda);
        }

        #[cfg(all(not(feature = "cuda"), feature = "hip"))]
        {
            assert_eq!(backends[0], BackendType::Hip);
        }

        #[cfg(all(not(feature = "cuda"), not(feature = "hip"), feature = "cpu"))]
        {
            assert_eq!(backends[0], BackendType::Cpu);
        }
    }

    #[test]
    fn test_engine_teardown_safety() {
        // INV-COMPUTE-003: explicit teardown verification
        #[cfg(feature = "cpu")]
        {
            let backend = instantiate_backend(BackendType::Cpu, None).unwrap();
            let layout = ShardLayout {
                padded_n: 64,
                total_axons: 100,
                total_ghosts: 10,
            };
            let mut engine = ShardEngine::new(backend, layout).unwrap();
            assert!(!engine.is_teared_down());

            // Teardown context
            engine.teardown().unwrap();
            assert!(engine.is_teared_down());
            // Rust's Drop will run next, testing R-023 double-panic/free safety
        }
    }

    #[test]
    fn test_double_teardown_safety() {
        // E-067: ignore repeat calls to teardown
        #[cfg(feature = "cpu")]
        {
            let backend = instantiate_backend(BackendType::Cpu, None).unwrap();
            let layout = ShardLayout {
                padded_n: 64,
                total_axons: 100,
                total_ghosts: 10,
            };
            let mut engine = ShardEngine::new(backend, layout).unwrap();

            // First teardown
            engine.teardown().unwrap();
            assert!(engine.is_teared_down());

            // Second teardown should be a safe no-op
            let res = engine.teardown();
            assert!(res.is_ok());
            assert!(engine.is_teared_down());
        }
    }

    #[test]
    fn test_stateless_facade_integrity() {
        // INV-COMPUTE-004: ensure ShardEngine remains stateless
        // The size of ShardEngine struct should be small, containing only the specified fields:
        // fat pointer for backend (16B), VramHandle (8B), ShardLayout (12B), bool (1B).
        // Max size on 64-bit platform with alignment padding is 48B.
        let size = std::mem::size_of::<ShardEngine>();
        assert!(size <= 48, "ShardEngine size must not exceed 48 bytes (size was {} B)", size);
    }

    #[test]
    fn test_instantiate_invalid_device() {
        // E-069: invalid device index triggers DeviceLost
        let res = instantiate_backend(BackendType::Cpu, Some(1));
        assert_eq!(res.err(), Some(ComputeApiError::DeviceLost));

        let res = instantiate_backend(BackendType::Cpu, Some(-1));
        assert_eq!(res.err(), Some(ComputeApiError::DeviceLost));
    }
}
