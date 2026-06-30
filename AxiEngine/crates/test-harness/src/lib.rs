//! Conformance and differential test harness for `AxiEngine` Layer 3 computational backends.

#![forbid(unsafe_code)]

#[cfg(feature = "abi")]
pub mod abi;
pub mod compare;
pub mod fixture;
#[cfg(feature = "mock")]
pub mod mock;
pub mod outcome;
pub mod runner;

#[cfg(feature = "abi")]
pub use abi::*;
pub use compare::*;
pub use fixture::*;
#[cfg(feature = "mock")]
pub use mock::*;
pub use outcome::*;
pub use runner::*;
