//! Pre-bake physics and anatomy validation (INV-BAKER-001, INV-BAKER-004).

use crate::error::BakerError;

/// Validates simulation physics parameters and cortical anatomy configuration
/// before the bake pipeline is allowed to proceed.
///
/// # Invariants
/// - **INV-BAKER-001 (Anatomy Integrity Guard)**: All `height_pct` values across layers
///   must sum to 1.0 (±1e-4). All `composition.share` values within each layer must
///   sum to 1.0 (±1e-4). Violation returns an immediate `Err` without partial recovery.
/// - **INV-BAKER-004 (Pre-Bake Guard)**: Physics validity is checked before any geometry
///   is generated. The function returns early via `?` at the first sign of mathematical
///   incorrectness. `unwrap`/`expect`/`panic` are strictly forbidden here.
///
/// # Errors
/// - [`BakerError::InvalidLayerHeights`] if `Σ height_pct` deviates from 1.0 by more than 1e-4.
/// - [`BakerError::InvalidComposition`] if any layer's `Σ share` deviates from 1.0 by more than 1e-4.
/// - [`BakerError::InvalidSignalSpeed`] if `physics::compute_v_seg` rejects the speed parameters.
pub fn validate_physics_and_anatomy(
    sim: &config::SimulationParams,
    anatomy: &config::AnatomyConfig,
) -> Result<(), BakerError> {
    // ── INV-BAKER-001: Validate sum of layer height percentages ──────────────
    let height_sum: f32 = anatomy.layers.iter().map(|l| l.height_pct).sum();
    if (height_sum - 1.0).abs() > 1e-4 {
        return Err(BakerError::InvalidLayerHeights { actual_sum: height_sum });
    }

    // ── INV-BAKER-001: Validate composition share sum per layer ──────────────
    for layer in &anatomy.layers {
        let share_sum: f32 = layer.composition.iter().map(|c| c.share).sum();
        if (share_sum - 1.0).abs() > 1e-4 {
            return Err(BakerError::InvalidComposition {
                layer_name: layer.name.clone(),
                actual_sum: share_sum,
            });
        }
    }

    // ── INV-BAKER-004: Validate Integer Physics speed constraint ─────────────
    physics::compute_v_seg(
        sim.signal_speed_m_s,
        sim.tick_duration_us,
        sim.voxel_size_um,
        sim.segment_length_voxels,
    )
    .map_err(|msg| BakerError::InvalidSignalSpeed(msg.to_string()))?;

    Ok(())
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use config::{AnatomyConfig, LayerConfig, NeuronTypeDistribution, SimulationParams};

    fn valid_sim() -> SimulationParams {
        SimulationParams {
            tick_duration_us: 1000,
            total_ticks: 0,
            master_seed: "test".to_string(),
            voxel_size_um: 10.0,
            segment_length_voxels: 2,
            signal_speed_m_s: 2.0, // v_seg = 2000 / 20 = 100 (integer ✓)
            sync_batch_ticks: 10,
            axon_growth_max_steps: 200,
            max_dendrites: 128,
        }
    }

    fn valid_anatomy() -> AnatomyConfig {
        AnatomyConfig {
            layers: vec![
                LayerConfig {
                    name: "L1".to_string(),
                    height_pct: 0.4,
                    density: 50000.0,
                    composition: vec![NeuronTypeDistribution {
                        type_name: "TypeA".to_string(),
                        share: 1.0,
                    }],
                },
                LayerConfig {
                    name: "L2".to_string(),
                    height_pct: 0.6,
                    density: 80000.0,
                    composition: vec![NeuronTypeDistribution {
                        type_name: "TypeB".to_string(),
                        share: 1.0,
                    }],
                },
            ],
        }
    }

    #[test]
    fn test_valid_params_pass() {
        let result = validate_physics_and_anatomy(&valid_sim(), &valid_anatomy());
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_height_sum() {
        let mut anatomy = valid_anatomy();
        anatomy.layers[0].height_pct = 0.3; // sum = 0.9 ≠ 1.0
        let result = validate_physics_and_anatomy(&valid_sim(), &anatomy);
        assert!(matches!(result, Err(BakerError::InvalidLayerHeights { .. })));
    }

    #[test]
    fn test_invalid_composition_share() {
        let mut anatomy = valid_anatomy();
        anatomy.layers[0].composition[0].share = 0.5; // sum = 0.5 ≠ 1.0
        let result = validate_physics_and_anatomy(&valid_sim(), &anatomy);
        assert!(matches!(result, Err(BakerError::InvalidComposition { .. })));
    }

    #[test]
    fn test_invalid_signal_speed_fractional() {
        let mut sim = valid_sim();
        sim.signal_speed_m_s = 1.23; // v_seg = 1230 / 20 = 61.5 (fractional ✗)
        let result = validate_physics_and_anatomy(&sim, &valid_anatomy());
        assert!(matches!(result, Err(BakerError::InvalidSignalSpeed(_))));
    }

    #[test]
    fn test_fail_fast_height_before_composition() {
        // Both height and composition are invalid; height error must surface first (Fail-Fast)
        let mut anatomy = valid_anatomy();
        anatomy.layers[0].height_pct = 0.1; // height sum = 0.7
        anatomy.layers[0].composition[0].share = 0.5; // composition also invalid
        let result = validate_physics_and_anatomy(&valid_sim(), &anatomy);
        assert!(matches!(result, Err(BakerError::InvalidLayerHeights { .. })));
    }
}
