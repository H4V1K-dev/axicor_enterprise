//! Error types for physics pre-bake and execution validation.

/// Errors that can occur during pre-bake physical parameter calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsError {
    /// Signal speed, voxel size, or other physical parameters are non-positive or invalid.
    InvalidPhysicalParameters,
    /// The calculated discrete segment velocity `v_seg` is non-exact (has a non-negligible fractional component).
    NonIntegerSegmentVelocity,
    /// The calculated discrete segment velocity `v_seg` is out of the valid range `1..=255`.
    SegmentVelocityOutOfBounds,
}
