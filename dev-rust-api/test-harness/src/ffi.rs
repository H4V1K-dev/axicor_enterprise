//! FFI Alignment verification module.
//!
//! Under invariant INV-CROSS-007, this module checks that the structure layouts,
//! sizes, and alignments of critical C-ABI FFI types match their expected values.

use core::mem::{size_of, align_of};
use types::PackedPosition;
use layout::{BurstHeads8, VariantParameters, ShardVramPtrs};

/// Verifies that all FFI structs adhere to the strict alignment and size requirements.
///
/// This function verifies:
/// - `PackedPosition` (Size 4, Align 4)
/// - `BurstHeads8` (Size 32, Align 32)
/// - `VariantParameters` (Size 64, Align 64)
/// - `ShardVramPtrs` (Size 10 * pointer size, Align pointer size)
///
/// Under invariant `INV-CROSS-007`, checking these constraints guarantees that the
/// structure layout is aligned with GPU hardware layout requirements and prevents memory
/// corruption during zero-copy host-to-device FFI transfers.
pub fn verify_ffi_alignments() {
    // Verify PackedPosition
    assert_eq!(size_of::<PackedPosition>(), 4, "PackedPosition size mismatch");
    assert_eq!(align_of::<PackedPosition>(), 4, "PackedPosition alignment mismatch");

    // Verify BurstHeads8
    assert_eq!(size_of::<BurstHeads8>(), 32, "BurstHeads8 size mismatch");
    assert_eq!(align_of::<BurstHeads8>(), 32, "BurstHeads8 alignment mismatch");

    // Verify VariantParameters
    assert_eq!(size_of::<VariantParameters>(), 64, "VariantParameters size mismatch");
    assert_eq!(align_of::<VariantParameters>(), 64, "VariantParameters alignment mismatch");

    // Verify ShardVramPtrs
    let ptr_size = size_of::<*mut u8>();
    assert_eq!(size_of::<ShardVramPtrs>(), 10 * ptr_size, "ShardVramPtrs size mismatch");
    assert_eq!(align_of::<ShardVramPtrs>(), align_of::<*mut u8>(), "ShardVramPtrs alignment mismatch");
}
