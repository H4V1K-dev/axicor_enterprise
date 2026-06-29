//! Layer 1 Memory Layouts and C-ABI Binary Contracts for `AxiEngine`.
//!
//! This crate defines the authoritative physical memory contracts, Structure of Arrays (SoA) layout formulas,
//! file dump headers, and VRAM pointer data transfer objects for simulation compute execution.
//! It is strictly integer and POD based, allocation-free (`no_std`), and ensures bit-for-bit zero-copy DMA compatibility.

#![no_std]

pub mod burst;
pub mod constants;
pub mod headers;
pub mod offsets;
pub mod variant;
pub mod vram;

pub use burst::*;
pub use constants::*;
pub use headers::*;
pub use offsets::*;
pub use variant::*;
pub use vram::*;
