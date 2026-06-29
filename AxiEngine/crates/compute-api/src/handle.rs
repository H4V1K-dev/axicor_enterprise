//! Opaque descriptor handle for allocated VRAM resources.

use crate::kind::BackendKind;
use core::num::NonZeroU64;

/// Opaque handle identifying an allocated simulation shard in VRAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VramHandle {
    kind: BackendKind,
    id: NonZeroU64,
    generation: u32,
}

impl VramHandle {
    /// Creates a new `VramHandle` from raw constituent parts.
    #[inline]
    pub const fn from_raw_parts(kind: BackendKind, id: NonZeroU64, generation: u32) -> Self {
        Self {
            kind,
            id,
            generation,
        }
    }

    /// Returns the backend kind associated with this handle.
    #[inline]
    pub const fn kind(&self) -> BackendKind {
        self.kind
    }

    /// Returns the non-zero allocation ID of this handle.
    #[inline]
    pub const fn id(&self) -> NonZeroU64 {
        self.id
    }

    /// Returns the allocation generation counter of this handle.
    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }
}
