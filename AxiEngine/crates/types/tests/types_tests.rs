use bytemuck::{Pod, Zeroable};
use static_assertions::{assert_impl_all, const_assert_eq};
use types::*;

// Suite 1: Compile-Time Size, Alignment, Trait & Const Fn Asserts
const_assert_eq!(core::mem::size_of::<PackedPosition>(), 4);
const_assert_eq!(core::mem::align_of::<PackedPosition>(), 4);

const_assert_eq!(core::mem::size_of::<PackedTarget>(), 4);
const_assert_eq!(core::mem::align_of::<PackedTarget>(), 4);

const_assert_eq!(core::mem::size_of::<SomaFlags>(), 1);
const_assert_eq!(core::mem::align_of::<SomaFlags>(), 1);

const_assert_eq!(core::mem::size_of::<MasterSeed>(), 8);
const_assert_eq!(core::mem::align_of::<MasterSeed>(), 8);

assert_impl_all!(PackedPosition: Pod, Zeroable);
assert_impl_all!(PackedTarget: Pod, Zeroable);
assert_impl_all!(SomaFlags: Pod, Zeroable);
assert_impl_all!(MasterSeed: Pod, Zeroable);

const CONST_POS: PackedPosition = PackedPosition::new(1, 2, 3, 4);
const CONST_TGT: PackedTarget = PackedTarget::pack(10, 5);

#[test]
fn test_const_fn_smoke() {
    assert_eq!(CONST_POS.x(), 1);
    assert_eq!(CONST_POS.y(), 2);
    assert_eq!(CONST_POS.z(), 3);
    assert_eq!(CONST_POS.type_id(), 4);

    assert_eq!(CONST_TGT.unpack(), Some((10, 5)));
}

// Suite 2 & 3: Pack / Unpack Roundtrip & Boundary Values Tests
#[test]
fn test_position_pack_unpack_boundary() {
    let pos = PackedPosition::new(MAX_VOXEL_X, MAX_VOXEL_Y, MAX_VOXEL_Z, MAX_TYPE_ID);
    assert_eq!(pos.x(), 1023);
    assert_eq!(pos.y(), 1023);
    assert_eq!(pos.z(), 255);
    assert_eq!(pos.type_id(), 15);
    assert_eq!(pos.0, 0xFFFF_FFFF);
}

#[test]
fn test_target_pack_unpack_boundary() {
    let tgt_min = PackedTarget::pack(0, 0);
    assert_eq!(tgt_min.0, 1);
    assert_eq!(tgt_min.unpack(), Some((0, 0)));

    let tgt_max = PackedTarget::pack(MAX_AXON_ID, MAX_SEGMENT_OFFSET);
    assert_eq!(tgt_max.0, 0xFFFF_FFFE);
    assert_eq!(tgt_max.unpack(), Some((MAX_AXON_ID, MAX_SEGMENT_OFFSET)));
    assert_ne!(tgt_max.0, EMPTY_PIXEL);
}

// Suite 4 & 5: Collision & Bit Bleed Tests (E-001)
#[test]
fn test_bit_bleed_and_bounds() {
    assert_eq!(
        PackedPosition::try_new(MAX_VOXEL_X + 1, 0, 0, 0),
        Err(TypeError::PositionOutOfBounds {
            x: MAX_VOXEL_X + 1,
            y: 0,
            z: 0,
            type_id: 0
        })
    );

    assert_eq!(
        PackedTarget::try_pack(MAX_AXON_ID + 1, 0),
        Err(TypeError::TargetOutOfBounds {
            axon_id: MAX_AXON_ID + 1,
            segment_offset: 0
        })
    );
}

// Suite 6: Target States & Unpack Safety Tests (E-002)
#[test]
fn test_target_states_and_safety() {
    let none = PackedTarget(0);
    assert!(none.is_zero_none());
    assert!(none.is_inactive());
    assert!(!none.is_active());
    assert!(none.is_valid_raw());
    assert_eq!(none.unpack(), None);
    assert_eq!(none.try_unpack(), Ok(None));

    let tombstone = PackedTarget(EMPTY_PIXEL);
    assert!(tombstone.is_tombstone());
    assert!(tombstone.is_inactive());
    assert!(!tombstone.is_active());
    assert!(tombstone.is_valid_raw());
    assert_eq!(tombstone.unpack(), None);
    assert_eq!(tombstone.try_unpack(), Ok(None));

    // Reserved / Corrupt encodings
    let corrupt1 = PackedTarget(0x00FF_FFFF);
    assert!(!corrupt1.is_valid_raw());
    assert!(corrupt1.is_reserved_encoding());
    assert_eq!(corrupt1.unpack(), None);
    assert_eq!(
        corrupt1.try_unpack(),
        Err(TypeError::CorruptTarget { raw: 0x00FF_FFFF })
    );

    let corrupt2 = PackedTarget(0xFEFF_FFFF);
    assert!(!corrupt2.is_valid_raw());
    assert!(corrupt2.is_reserved_encoding());
    assert_eq!(corrupt2.unpack(), None);
    assert_eq!(
        corrupt2.try_unpack(),
        Err(TypeError::CorruptTarget { raw: 0xFEFF_FFFF })
    );

    // Underflow protection check
    let corrupt_underflow = PackedTarget(0x0100_0000);
    assert!(!corrupt_underflow.is_valid_raw());
    assert_eq!(corrupt_underflow.unpack(), None);
    assert_eq!(
        corrupt_underflow.try_unpack(),
        Err(TypeError::CorruptTarget { raw: 0x0100_0000 })
    );
}

// Suite 7: SomaFlags Accessors & Saturating Clamp Tests (E-003)
#[test]
fn test_soma_flags() {
    let mut flags = SomaFlags::new(true, 5, 12);
    assert!(flags.spiking());
    assert_eq!(flags.burst_count(), 5);
    assert_eq!(flags.type_id(), 12);

    flags.set_spiking(false);
    assert!(!flags.spiking());
    assert_eq!(flags.type_id(), 12);

    flags.set_burst_count(10); // clamped to 7
    assert_eq!(flags.burst_count(), 7);
    assert_eq!(flags.type_id(), 12);
}

// Suite 8: Deterministic Seed & Hash Tests (E-004)
#[test]
fn test_golden_hashes() {
    assert_eq!(hash_name_fnv1a(b"SensoryCortex"), 0x273fd103);
    assert_eq!(fnv1a_32(b"SensoryCortex"), 0x273fd103);

    assert_eq!(seed_from_str("AXICOR").0, 0x0d7388e891ead1f9);
    assert_eq!(entity_seed(MasterSeed(0), 1), 0x0d603133dc4196d3);
}

// Suite 9: Integer RNG Boundary Tests (E-005)
#[test]
fn test_golden_rng() {
    let master = MasterSeed(0);
    assert_eq!(master.random_u64(0), 0xdfdf403e8fd5912b);
    assert_eq!(master.random_u32(0), 0xdfdf403e);

    let master_complex = MasterSeed(0x1234_5678_9ABC_DEF0);
    assert_eq!(master_complex.random_u32(42), 0x2032_dc07);
}
