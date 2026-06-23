//! Axon signal propagation and active tail dynamics.

/// Computes the initial head position of a newly spawned axon spike.
///
/// # E-009 Spike Birth Invariant
/// The newly born signal head is initialized using wrap-around arithmetic as
/// `0u32.wrapping_sub(v_seg)`. On the subsequent propagation step, adding `v_seg`
/// yields exactly `0`, guaranteeing that segment 0 is not skipped and is correctly
/// scanned by target dendrites.
#[inline]
pub const fn initial_axon_head(v_seg: u32) -> u32 {
    0u32.wrapping_sub(v_seg)
}

/// Checks if a given axon segment lies within the active signal propagation front (active tail).
///
/// Returns `true` if `head.wrapping_sub(segment_idx) < prop_len`.
#[inline]
pub const fn is_in_active_tail(head: u32, segment_idx: u32, prop_len: u32) -> bool {
    head.wrapping_sub(segment_idx) < prop_len
}

/// Legacy axon head initialization.
///
/// In the legacy `signal.rs`, the head position was initialized as
/// `AXON_SENTINEL - length_segments * v_seg`.
#[inline]
pub const fn initial_axon_head_legacy(length_segments: u32, v_seg: u32) -> u32 {
    types::AXON_SENTINEL.wrapping_sub(length_segments.wrapping_mul(v_seg))
}

/// Legacy active tail check containing explicit sentinel checks.
#[inline]
pub const fn is_segment_active_legacy(axon_head: u32, segment_idx: u32, propagation_length: u32) -> bool {
    if axon_head == types::AXON_SENTINEL {
        return false;
    }
    axon_head.wrapping_sub(segment_idx) < propagation_length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_axon_head_and_active_tail() {
        use types::AXON_SENTINEL;
        let head = initial_axon_head_legacy(10, 1);
        assert_ne!(head, AXON_SENTINEL);
        assert_ne!(head, 0);

        // test mid flight
        let length = 5u32;
        let propagation = 3u32;
        let mut h = initial_axon_head_legacy(length, 1);
        for _ in 0..(length - propagation) {
            h = h.wrapping_add(1);
        }
        let starting_segment = AXON_SENTINEL.wrapping_sub(length * 1);
        assert!(is_segment_active_legacy(h, starting_segment, propagation));
    }

    #[test]
    fn test_spike_birth_wrap_around() {
        // E-009: Verify that adding v_seg back to the initial head position wraps to exactly 0.
        for v_seg in [1, 2, 5, 10, 255] {
            let initial = initial_axon_head(v_seg);
            assert_eq!(initial.wrapping_add(v_seg), 0);
        }
    }

    #[test]
    fn test_is_in_active_tail() {
        // active tail spanning segments 0..3 (prop_len = 4, head = 3)
        assert!(is_in_active_tail(3, 3, 4)); // segment 3 is the head
        assert!(is_in_active_tail(3, 2, 4));
        assert!(is_in_active_tail(3, 0, 4)); // segment 0 is still within prop_len
        assert!(!is_in_active_tail(3, 4, 4)); // segment 4 has not been reached yet
        assert!(!is_in_active_tail(3, 5, 4));
    }

    #[test]
    fn test_sentinel_edge_case() {
        // INV-PHYS-006: Inactive axon head (AXON_SENTINEL) must never fall inside active tail
        use types::AXON_SENTINEL;
        for segment_idx in 0..256 {
            assert!(!is_in_active_tail(AXON_SENTINEL, segment_idx, 4));
        }
    }

    #[test]
    fn test_magnetic_sentinel_zombie_freeze() {
        // INV-PHYS-006: Zombie spikes trying to bypass AXON_SENTINEL must be frozen at zero step
        use types::AXON_SENTINEL;
        let v_seg = 2;

        // If head is at sentinel, step must be 0
        let h_sentinel = AXON_SENTINEL;
        let step_sentinel = v_seg * (((h_sentinel ^ AXON_SENTINEL) >= v_seg) as u32);
        assert_eq!(step_sentinel, 0);

        // If head overflows sentinel (zombie), step must be 0 (frozen)
        let h_zombie = AXON_SENTINEL + 1;
        let step_zombie = v_seg * (((h_zombie ^ AXON_SENTINEL) >= v_seg) as u32);
        assert_eq!(step_zombie, 0);

        // Normal propagation step is v_seg when head is far from sentinel
        let h_normal = 100;
        let step_normal = v_seg * (((h_normal ^ AXON_SENTINEL) >= v_seg) as u32);
        assert_eq!(step_normal, v_seg);
    }

    #[test]
    fn test_burst_compression_bitwise_or() {
        // INV-PHYS-001: Combining overlapping hits branchless using bitwise OR
        let h0 = 5;
        let h1 = 12;
        let seg_idx = 4;
        let prop_len = 3;

        let hit_0 = is_in_active_tail(h0, seg_idx, prop_len); // true (5 - 4 = 1 < 3)
        let hit_1 = is_in_active_tail(h1, seg_idx, prop_len); // false (12 - 4 = 8 >= 3)

        let combined_hit = (hit_0 as u32) | (hit_1 as u32);
        assert_eq!(combined_hit, 1);
    }
}
