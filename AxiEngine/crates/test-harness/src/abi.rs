//! Static ABI mirror alignment and size verification.

use crate::outcome::HarnessErrorKind;

/// Verifies that sizes and alignments of C-ABI layouts match compile-time expectations.
pub fn verify_abi_mirrors() -> Result<(), HarnessErrorKind> {
    // 1. ShardVramPtrs
    let size_vram_ptrs = core::mem::size_of::<layout::ShardVramPtrs>();
    let align_vram_ptrs = core::mem::align_of::<layout::ShardVramPtrs>();
    let ptr_size = core::mem::size_of::<*mut u8>();
    let ptr_align = core::mem::align_of::<*mut u8>();

    if size_vram_ptrs != 9 * ptr_size {
        return Err(HarnessErrorKind::AbiMirrorMismatch {
            struct_name: "ShardVramPtrs",
            reason: "Expected size of ShardVramPtrs to be 9 * pointer size",
        });
    }
    if align_vram_ptrs != ptr_align {
        return Err(HarnessErrorKind::AbiMirrorMismatch {
            struct_name: "ShardVramPtrs",
            reason: "Expected alignment of ShardVramPtrs to match pointer alignment",
        });
    }

    // 2. BurstHeads8
    let size_burst_heads = core::mem::size_of::<layout::BurstHeads8>();
    let align_burst_heads = core::mem::align_of::<layout::BurstHeads8>();
    if size_burst_heads != 32 {
        return Err(HarnessErrorKind::AbiMirrorMismatch {
            struct_name: "BurstHeads8",
            reason: "Expected size of BurstHeads8 to be 32 bytes",
        });
    }
    if align_burst_heads != 32 {
        return Err(HarnessErrorKind::AbiMirrorMismatch {
            struct_name: "BurstHeads8",
            reason: "Expected alignment of BurstHeads8 to be 32 bytes",
        });
    }

    // 3. VariantParameters
    let size_variant = core::mem::size_of::<layout::VariantParameters>();
    let align_variant = core::mem::align_of::<layout::VariantParameters>();
    if size_variant != 64 {
        return Err(HarnessErrorKind::AbiMirrorMismatch {
            struct_name: "VariantParameters",
            reason: "Expected size of VariantParameters to be 64 bytes",
        });
    }
    if align_variant != 64 {
        return Err(HarnessErrorKind::AbiMirrorMismatch {
            struct_name: "VariantParameters",
            reason: "Expected alignment of VariantParameters to be 64 bytes",
        });
    }

    Ok(())
}
