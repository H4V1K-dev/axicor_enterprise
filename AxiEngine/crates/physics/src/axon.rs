//! Axonal signal propagation and active tail contact algorithms.

use types::AXON_SENTINEL;

/// Initializes the axonal propagation head for a newly born spike.
///
/// Implements `h0 = 0u32.wrapping_sub(v_seg)` to prevent temporal paradoxes, ensuring that
/// after the first step of propagation (`propagate_head`), the head arrives exactly at segment 0.
#[inline]
pub fn initial_axon_head(v_seg: u32) -> u32 {
    0u32.wrapping_sub(v_seg)
}

/// Advances an axonal propagation head by `v_seg` segments while enforcing the Magnetic Sentinel Trap.
///
/// Enforces `(head ^ AXON_SENTINEL) >= v_seg` to prevent active heads with `v_seg > 1` from
/// hopping over `AXON_SENTINEL`. Inactive heads remain locked at `AXON_SENTINEL`.
/// Implements branchless mask selection adhering strictly to `INV-PHYS-001`.
#[inline]
pub fn propagate_head(head: u32, v_seg: u32) -> u32 {
    let is_active = (head ^ AXON_SENTINEL) >= v_seg;
    let mask = 0u32.wrapping_sub(is_active as u32);
    (head.wrapping_add(v_seg) & mask) | (AXON_SENTINEL & !mask)
}

/// Evaluates whether a dendrite reading segment contacts any active tail in an array of 8 burst heads.
///
/// # Arguments
/// * `heads` - Array of 8 buffered axonal burst heads (`[u32; 8]`).
/// * `seg_idx` - Target segment index on the axon.
/// * `propagation_length` - Signal propagation tail length ($L_{\text{prop}}$).
///
/// # Returns
/// `true` if any head satisfies `head.wrapping_sub(seg_idx) < propagation_length`, `false` otherwise.
/// Implements branchless loop accumulation adhering strictly to `INV-PHYS-001`.
pub fn active_tail_hit(heads: &[u32; 8], seg_idx: u32, propagation_length: u32) -> bool {
    let mut hit_mask = 0u32;
    let mut i = 0;
    while i < 8 {
        let d = heads[i].wrapping_sub(seg_idx);
        hit_mask |= (d < propagation_length) as u32;
        i += 1;
    }
    hit_mask != 0
}
