//! Synaptic pruning (§6.1) and dendrite compaction (§6.2) algorithms.

// ─────────────────────────────────────────────────────────────────────────────
// prune_synapses
// ─────────────────────────────────────────────────────────────────────────────

/// Prunes synapses whose absolute weight falls below the given threshold.
///
/// # Algorithm (§6.1)
/// For each synapse slot `i`:
/// 1. Compute the Mass-Domain threshold: `threshold_mass = (threshold as i32) << 16`.
/// 2. If `weights[i].unsigned_abs() < threshold_mass as u32`:
///    - Zero the weight: `weights[i] = 0`.
///    - Invalidate the target slot: `targets[i] = types::EMPTY_PIXEL` (E-093, E-094).
///
/// # Arguments
/// * `weights`   — mutable slice of synaptic weights in Mass Domain (`i32`).
/// * `targets`   — mutable slice of packed dendrite target IDs (`u32`).
/// * `threshold` — pruning threshold in base domain (`i16`); shifted left by 16 bits
///                 before comparison to produce the Mass-Domain threshold.
pub fn prune_synapses(weights: &mut [i32], targets: &mut [u32], threshold: i16) {
    debug_assert_eq!(
        weights.len(),
        targets.len(),
        "prune_synapses: weights and targets slices must have equal length"
    );

    // Shift threshold into Mass Domain (§6.1)
    let threshold_mass = (threshold as i32) << 16;
    let threshold_abs = threshold_mass.unsigned_abs();

    for i in 0..weights.len() {
        if weights[i].unsigned_abs() < threshold_abs {
            weights[i] = 0;
            targets[i] = types::EMPTY_PIXEL; // E-093, E-094
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// compact_dendrites
// ─────────────────────────────────────────────────────────────────────────────

/// Defragments SoA dendrite arrays by evicting `EMPTY_PIXEL` slots to the right.
///
/// # INV-WDAEMON-003 (Align Integrity during Compaction)
/// The SoA dendrite arrays are laid out in columnar format with stride `padded_n`:
/// the synapse at neuron `n`, slot `s` is located at index `s * padded_n + n`.
/// Compaction operates per-neuron: for each neuron `n`, the `slots`-wide block
/// is compacted in-place so that all valid (non-`EMPTY_PIXEL`) entries are
/// shifted to the front of the block and all `EMPTY_PIXEL` entries are pushed
/// to the tail. The columnar stride is preserved throughout — no re-layout occurs.
///
/// # Arguments
/// * `weights`  — mutable SoA weight array (columnar, stride = `padded_n`).
/// * `targets`  — mutable SoA target array (columnar, stride = `padded_n`).
/// * `padded_n` — number of neurons (column stride), must be > 0.
/// * `slots`    — number of dendrite slots per neuron (typically 128).
pub fn compact_dendrites(
    weights: &mut [i32],
    targets: &mut [u32],
    padded_n: usize,
    slots: usize,
) {
    debug_assert!(padded_n > 0, "compact_dendrites: padded_n must be > 0");
    debug_assert_eq!(
        weights.len(),
        targets.len(),
        "compact_dendrites: weights and targets slices must have equal length"
    );

    // Outer loop: iterate over each neuron column (INV-WDAEMON-003)
    for n in 0..padded_n {
        // write_idx tracks the next free slot at the front of the neuron's block
        let mut write_idx = 0usize;

        // Inner loop: scan all slots for this neuron (read pass)
        for read_idx in 0..slots {
            let src = read_idx * padded_n + n;
            if targets[src] != types::EMPTY_PIXEL {
                // Valid synapse — move it to the front
                if write_idx != read_idx {
                    let dst = write_idx * padded_n + n;
                    weights[dst] = weights[src];
                    targets[dst] = targets[src];
                }
                write_idx += 1;
            }
        }

        // Fill trailing slots with EMPTY_PIXEL / zero weight
        for tail_idx in write_idx..slots {
            let dst = tail_idx * padded_n + n;
            weights[dst] = 0;
            targets[dst] = types::EMPTY_PIXEL;
        }
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── test_pruning_by_threshold ─────────────────────────────────────────────

    /// E-093 / E-094: Pruned weights must be zeroed and targets set to EMPTY_PIXEL.
    #[test]
    fn test_pruning_by_threshold() {
        // threshold = 1 → threshold_mass = 1 << 16 = 65536
        // Weights below 65536 in abs must be pruned; at or above must survive.
        let mut weights = vec![
            65535,   // abs < 65536 → pruned
            65536,   // abs == 65536 → survives (not strictly less)
            -65535,  // abs < 65536 → pruned
            -100000, // abs > 65536 → survives
            0,       // abs == 0 → pruned
        ];
        let mut targets = vec![10u32, 20u32, 30u32, 40u32, 50u32];

        prune_synapses(&mut weights, &mut targets, 1);

        // Pruned slots
        assert_eq!(weights[0], 0);
        assert_eq!(targets[0], types::EMPTY_PIXEL);
        assert_eq!(weights[2], 0);
        assert_eq!(targets[2], types::EMPTY_PIXEL);
        assert_eq!(weights[4], 0);
        assert_eq!(targets[4], types::EMPTY_PIXEL);

        // Surviving slots
        assert_eq!(weights[1], 65536);
        assert_eq!(targets[1], 20);
        assert_eq!(weights[3], -100000);
        assert_eq!(targets[3], 40);
    }

    #[test]
    fn test_pruning_threshold_zero_prunes_nothing() {
        // threshold = 0 → threshold_mass = 0 << 16 = 0 → nothing pruned (abs >= 0 always)
        let mut weights = vec![0i32, 1, -1, 100];
        let mut targets = vec![1u32, 2u32, 3u32, 4u32];
        let original_weights = weights.clone();
        let original_targets = targets.clone();

        prune_synapses(&mut weights, &mut targets, 0);

        assert_eq!(weights, original_weights);
        assert_eq!(targets, original_targets);
    }

    // ── test_dendrite_compaction_logic ────────────────────────────────────────

    /// INV-WDAEMON-003: After compaction, valid slots must be at the front and
    /// EMPTY_PIXEL slots at the tail, preserving columnar SoA layout.
    #[test]
    fn test_dendrite_compaction_logic() {
        // 2 neurons (padded_n=2), 4 slots each.
        // Neuron 0 (column 0): slots [VALID, EMPTY, VALID, EMPTY]
        // Neuron 1 (column 1): slots [EMPTY, VALID, EMPTY, VALID]
        //
        // Columnar layout (slot * padded_n + n):
        // Index: 0  1  2  3  4  5  6  7
        //         n0 n1 n0 n1 n0 n1 n0 n1
        //  slot:   0  0  1  1  2  2  3  3
        let ep = types::EMPTY_PIXEL;
        let padded_n = 2;
        let slots = 4;

        // targets layout (columnar): slot0_n0, slot0_n1, slot1_n0, slot1_n1, ...
        let mut targets = vec![
            10u32, ep,    // slot 0: n0=valid(10), n1=empty
            ep,    20u32, // slot 1: n0=empty, n1=valid(20)
            30u32, ep,    // slot 2: n0=valid(30), n1=empty
            ep,    40u32, // slot 3: n0=empty, n1=valid(40)
        ];
        let mut weights = vec![
            100i32, 0,   // slot 0
            0,      200, // slot 1
            300,    0,   // slot 2
            0,      400, // slot 3
        ];

        compact_dendrites(&mut weights, &mut targets, padded_n, slots);

        // After compaction for neuron 0 (n=0):
        // Slots 0..4 at indices 0,2,4,6 → valid: 10,30; empty: ep,ep
        assert_eq!(targets[0], 10);  // slot 0, n0 — valid front
        assert_eq!(targets[2], 30);  // slot 1, n0 — valid front
        assert_eq!(targets[4], ep);  // slot 2, n0 — empty tail
        assert_eq!(targets[6], ep);  // slot 3, n0 — empty tail
        assert_eq!(weights[0], 100);
        assert_eq!(weights[2], 300);
        assert_eq!(weights[4], 0);
        assert_eq!(weights[6], 0);

        // After compaction for neuron 1 (n=1):
        // Slots at indices 1,3,5,7 → valid: 20,40; empty: ep,ep
        assert_eq!(targets[1], 20);  // slot 0, n1 — valid front
        assert_eq!(targets[3], 40);  // slot 1, n1 — valid front
        assert_eq!(targets[5], ep);  // slot 2, n1 — empty tail
        assert_eq!(targets[7], ep);  // slot 3, n1 — empty tail
        assert_eq!(weights[1], 200);
        assert_eq!(weights[3], 400);
        assert_eq!(weights[5], 0);
        assert_eq!(weights[7], 0);
    }

    #[test]
    fn test_compaction_all_valid_no_change() {
        // All slots filled — compaction must not alter anything
        let padded_n = 2;
        let slots = 2;
        let mut targets = vec![1u32, 2u32, 3u32, 4u32];
        let mut weights = vec![10i32, 20i32, 30i32, 40i32];
        let orig_t = targets.clone();
        let orig_w = weights.clone();

        compact_dendrites(&mut weights, &mut targets, padded_n, slots);

        assert_eq!(targets, orig_t);
        assert_eq!(weights, orig_w);
    }

    #[test]
    fn test_compaction_all_empty_stays_empty() {
        let padded_n = 2;
        let slots = 2;
        let ep = types::EMPTY_PIXEL;
        let mut targets = vec![ep, ep, ep, ep];
        let mut weights = vec![0i32; 4];

        compact_dendrites(&mut weights, &mut targets, padded_n, slots);

        assert!(targets.iter().all(|&t| t == ep));
        assert!(weights.iter().all(|&w| w == 0));
    }
}
