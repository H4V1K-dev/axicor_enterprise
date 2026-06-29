//! Validation errors for packed ABI types and boundary constructors.

/// Validation errors for packed types at system boundaries (Checked Constructors / `try_*` methods).
/// Lightweight `no_std` / `no_alloc` enum without dynamic allocations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeError {
    /// Position coordinates or type_id exceed allowed maximum values.
    PositionOutOfBounds {
        /// X coordinate provided.
        x: u32,
        /// Y coordinate provided.
        y: u32,
        /// Z coordinate provided.
        z: u32,
        /// Type ID provided.
        type_id: u8,
    },
    /// Target axon_id or segment_offset exceed allowed maximum values.
    TargetOutOfBounds {
        /// Axon ID provided.
        axon_id: u32,
        /// Segment offset provided.
        segment_offset: u32,
    },
    /// PackedTarget contains a corrupt or reserved bit encoding.
    CorruptTarget {
        /// Raw u32 bit representation.
        raw: u32,
    },
}
