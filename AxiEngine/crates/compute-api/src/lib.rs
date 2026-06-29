//! Hardware Abstraction Layer (HAL) and binary contracts for simulation compute execution.
//!
//! This crate defines the hardware-independent contracts (`ComputeBackend`), DTO payloads,
//! execution capabilities, opaque VRAM descriptors (`VramHandle`), and unified error types.
//! It is strictly allocation-free (`no_std`), safe, and provides validation utilities for compute backends.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

pub mod backend;
pub mod capabilities;
pub mod dto;
pub mod error;
pub mod handle;
pub mod kind;
pub mod validation;

pub use backend::*;
pub use capabilities::*;
pub use dto::*;
pub use error::*;
pub use handle::*;
pub use kind::*;
pub use validation::*;
