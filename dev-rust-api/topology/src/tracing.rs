use rand::Rng;
use rand_chacha::ChaCha8Rng;
use types::PackedPosition;
use crate::types::{LivingAxon, GhostPacket, GrowthEvent};

/// Weights parameterizing steering dynamics towards the target region.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SteeringWeights {
    pub global: f32,
    pub noise: f32,
}

/// Helper generating a random normalized 3D vector.
///
/// Uses deterministic rejection sampling inside a unit sphere to ensure uniform distribution
/// and bit-to-bit reproducibility across all platforms.
fn random_dir(rng: &mut ChaCha8Rng) -> glam::Vec3 {
    loop {
        let x = rng.gen_range(-1.0..=1.0);
        let y = rng.gen_range(-1.0..=1.0);
        let z = rng.gen_range(-1.0..=1.0);
        let v = glam::Vec3::new(x, y, z);
        let len_sq = v.length_squared();
        if len_sq > 0.0 && len_sq <= 1.0 {
            return v.normalize();
        }
    }
}

/// Traces the initial geometric path (Trunk Phase) of an axon towards a target Z layer.
///
/// # Arguments
/// * `soma_pos` - Packed position of the source soma.
/// * `target_z` - Target Z coordinate in voxels.
/// * `weights` - Steering parameters combining global direction and stochastic jitter.
/// * `segment_length_voxels` - Length of each growing segment in voxel units.
/// * `rng` - Deterministic random number generator.
pub fn cone_tracing(
    soma_pos: types::PackedPosition,
    target_z: u32,
    weights: &SteeringWeights,
    segment_length_voxels: f32,
    rng: &mut ChaCha8Rng,
) -> Vec<types::PackedPosition> {
    let mut segments = Vec::with_capacity(layout::MAX_SEGMENTS_PER_AXON);
    let mut current_f32_pos = glam::Vec3::new(
        soma_pos.x() as f32,
        soma_pos.y() as f32,
        soma_pos.z() as f32,
    );
    let type_mask = soma_pos.type_id();

    for _ in 0..layout::MAX_SEGMENTS_PER_AXON {
        // Compute steering direction: global vector towards target Z combined with noise
        let v_target = glam::Vec3::new(current_f32_pos.x, current_f32_pos.y, target_z as f32);
        let v_global = (v_target - current_f32_pos).normalize_or_zero();
        let v_noise = random_dir(rng);
        
        let v_steer = (v_global * weights.global + v_noise * weights.noise).normalize_or_zero();
        
        // Take a step in continuous 3D space
        current_f32_pos += v_steer * segment_length_voxels;

        // Quantize coordinates to discrete voxel space clamping to shard limits
        let x = (current_f32_pos.x.round().max(0.0) as u32).min(1023);
        let y = (current_f32_pos.y.round().max(0.0) as u32).min(1023);
        let z = (current_f32_pos.z.round().max(0.0) as u32).min(255);

        let packed = PackedPosition::pack_raw(x, y, z, type_mask);

        // Stagnation Guard: terminate growth if the quantized coordinate doesn't advance
        if let Some(&last) = segments.last() {
            if packed == last {
                break;
            }
        }

        segments.push(packed);

        // Early Exit: stop tracing when target layer Z level is reached
        if z == target_z {
            break;
        }
    }

    segments
}

/// Checks if the continuous coordinate vector exceeds physical shard boundaries.
pub fn is_out_of_bounds(pos: &glam::Vec3, bounds: (u32, u32, u32)) -> bool {
    let (max_x, max_y, max_z) = bounds;
    pos.x < 0.0 || pos.x >= max_x as f32 ||
    pos.y < 0.0 || pos.y >= max_y as f32 ||
    pos.z < 0.0 || pos.z >= max_z as f32
}

/// Advances the growth cones of active local axons and all ghost axons by one step.
///
/// # Invariants
/// - **INV-TOPO-009**: Activity-Based Nudging.
///   A local axon can grow during the Night Phase only if its owning soma spiked during
///   the preceding Day Phase (checked via `soma_flags` 0-th bit).
/// - **INV-TOPO-010**: Inertial Nudging for Ghost axons.
///   Ghost axons do not have a local soma (`soma_idx == usize::MAX`) and grow every night
///   unconditionally along their vector of inertia until `remaining_steps` reaches 0.
pub fn nudge_living_axons(
    living: &mut [LivingAxon],
    soma_flags: &[u8],
    bounds: (u32, u32, u32),
    segment_length_voxels: f32,
) -> Vec<GrowthEvent> {
    let mut events = Vec::with_capacity(living.len());

    for axon in living.iter_mut() {
        if axon.remaining_steps == 0 {
            continue;
        }

        let is_ghost = axon.soma_idx == usize::MAX;
        
        // INV-TOPO-009 & INV-TOPO-010: Check if growth is gated
        let should_grow = is_ghost || {
            (soma_flags[axon.soma_idx] & 0x01) != 0
        };

        if !should_grow {
            axon.last_night_active = false;
            continue;
        }

        axon.last_night_active = true;

        let current_pos = PackedPosition(axon.tip_uvw);
        let current_f32 = glam::Vec3::new(
            current_pos.x() as f32,
            current_pos.y() as f32,
            current_pos.z() as f32,
        );
        let next_pos_f32 = current_f32 + (axon.forward_dir * segment_length_voxels);

        // Check if next step exits the physical bounds of this shard
        if is_out_of_bounds(&next_pos_f32, bounds) {
            let remaining = axon.remaining_steps.saturating_sub(1);
            axon.remaining_steps = 0; // Terminate local growth
            
            let type_idx = if is_ghost {
                0
            } else {
                current_pos.type_id() as usize
            };

            let entry_x = (next_pos_f32.x.round().max(0.0) as u32).min(1023);
            let entry_y = (next_pos_f32.y.round().max(0.0) as u32).min(1023);
            let entry_z = (next_pos_f32.z.round().max(0.0) as u32).min(255);

            events.push(GrowthEvent::OutOfBounds(GhostPacket {
                origin_shard_id: 0,
                soma_idx: axon.soma_idx,
                type_idx,
                entry_x,
                entry_y,
                entry_z,
                entry_dir: axon.forward_dir,
                remaining_steps: remaining,
            }));
        } else {
            let next_packed = PackedPosition::pack_raw(
                (next_pos_f32.x.round().max(0.0) as u32).min(1023),
                (next_pos_f32.y.round().max(0.0) as u32).min(1023),
                (next_pos_f32.z.round().max(0.0) as u32).min(255),
                current_pos.type_id(),
            );
            
            axon.tip_uvw = next_packed.0;
            axon.remaining_steps -= 1;
            events.push(GrowthEvent::Advanced(axon.tip_uvw));
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_random_dir_normalized() {
        let mut rng = ChaCha8Rng::seed_from_u64(1337);
        for _ in 0..100 {
            let dir = random_dir(&mut rng);
            assert!((dir.length() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn test_cone_tracing_basic() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let soma_pos = PackedPosition::pack_raw(10, 10, 5, 2);
        let weights = SteeringWeights { global: 1.0, noise: 0.1 };
        
        let path = cone_tracing(soma_pos, 20, &weights, 1.0, &mut rng);
        assert!(!path.is_empty());
        
        // The last element should have reached the target Z
        let last = path.last().unwrap();
        assert_eq!(last.z(), 20);
        assert_eq!(last.type_id(), 2);
    }

    #[test]
    fn test_cone_tracing_stagnation() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let soma_pos = PackedPosition::pack_raw(10, 10, 5, 1);
        let weights = SteeringWeights { global: 1.0, noise: 0.0 };
        
        // Zero segment length will cause instant stagnation on the second step
        let path = cone_tracing(soma_pos, 20, &weights, 0.0, &mut rng);
        assert_eq!(path.len(), 1);
    }

    #[test]
    fn test_cone_tracing_max_segments() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let soma_pos = PackedPosition::pack_raw(500, 500, 128, 0);
        let weights = SteeringWeights { global: 0.0, noise: 1.0 };
        
        // Step size 1.8 guarantees we never stagnate, target Z = 300 is unreachable
        let path = cone_tracing(soma_pos, 300, &weights, 1.8, &mut rng);
        assert_eq!(path.len(), layout::MAX_SEGMENTS_PER_AXON);
    }

    #[test]
    fn test_nudge_living_axons_activity_gate_and_ghosts() {
        let soma_pos = PackedPosition::pack_raw(50, 50, 50, 1);
        
        let mut axons = vec![
            // Axon 0: Local axon whose soma is active (spiked)
            LivingAxon {
                axon_id: 100,
                soma_idx: 0,
                tip_uvw: soma_pos.0,
                forward_dir: glam::Vec3::new(0.0, 0.0, 1.0),
                remaining_steps: 5,
                last_night_active: false,
            },
            // Axon 1: Local axon whose soma is inactive (didn't spike)
            LivingAxon {
                axon_id: 101,
                soma_idx: 1,
                tip_uvw: soma_pos.0,
                forward_dir: glam::Vec3::new(0.0, 0.0, 1.0),
                remaining_steps: 5,
                last_night_active: false,
            },
            // Axon 2: Ghost axon (soma_idx == usize::MAX), should grow unconditionally
            LivingAxon {
                axon_id: 102,
                soma_idx: usize::MAX,
                tip_uvw: soma_pos.0,
                forward_dir: glam::Vec3::new(0.0, 0.0, 1.0),
                remaining_steps: 5,
                last_night_active: false,
            },
        ];

        // active = 0x01, inactive = 0x00
        let soma_flags = vec![0x01, 0x00];
        let bounds = (100, 100, 100);
        
        let events = nudge_living_axons(&mut axons, &soma_flags, bounds, 1.0);
        
        assert_eq!(events.len(), 2); // Axon 0 and Axon 2
        
        // Axon 0 advanced
        assert!(axons[0].last_night_active);
        assert_eq!(axons[0].remaining_steps, 4);
        assert_eq!(PackedPosition(axons[0].tip_uvw).z(), 51);
        
        // Axon 1 skipped
        assert!(!axons[1].last_night_active);
        assert_eq!(axons[1].remaining_steps, 5);
        assert_eq!(PackedPosition(axons[1].tip_uvw).z(), 50);

        // Axon 2 advanced (Ghost)
        assert!(axons[2].last_night_active);
        assert_eq!(axons[2].remaining_steps, 4);
        assert_eq!(PackedPosition(axons[2].tip_uvw).z(), 51);
    }

    #[test]
    fn test_nudge_living_axons_out_of_bounds() {
        let soma_pos = PackedPosition::pack_raw(99, 99, 99, 3);
        
        let mut axons = vec![
            // Growing out of bounds along Z axis
            LivingAxon {
                axon_id: 200,
                soma_idx: 0,
                tip_uvw: soma_pos.0,
                forward_dir: glam::Vec3::new(0.0, 0.0, 1.0),
                remaining_steps: 10,
                last_night_active: false,
            },
        ];

        let soma_flags = vec![0x01];
        let bounds = (100, 100, 100);
        
        let events = nudge_living_axons(&mut axons, &soma_flags, bounds, 1.0);
        
        assert_eq!(events.len(), 1);
        assert_eq!(axons[0].remaining_steps, 0); // Local growth terminated
        
        match &events[0] {
            GrowthEvent::OutOfBounds(packet) => {
                assert_eq!(packet.soma_idx, 0);
                assert_eq!(packet.type_idx, 3);
                assert_eq!(packet.entry_z, 100); // 99 + 1
                assert_eq!(packet.remaining_steps, 9); // 10 - 1
                assert_eq!(packet.entry_dir, glam::Vec3::new(0.0, 0.0, 1.0));
            }
            other => panic!("Expected OutOfBounds event, got {:?}", other),
        }
    }
}
