//! Layer 0 Primitives and Packed ABI Contracts for `AxiEngine`.
//!
//! This crate provides the foundational integer vocabulary, packed ABI layouts,
//! deterministic hashing algorithms, and hardware constants for the AxiEngine simulation ecosystem.
//! It is strictly integer-based, allocation-free (`no_std`), and side-effect free.

#![no_std]

pub mod constants;
pub mod error;
pub mod flags;
pub mod hash;
pub mod position;
pub mod primitives;
pub mod seed;
pub mod target;

pub use constants::*;
pub use error::*;
pub use flags::*;
pub use hash::*;
pub use position::*;
pub use primitives::*;
pub use seed::*;
pub use target::*;
