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

#[cfg(feature = "mvp-cpu-replay")]
pub mod mvp_cpu_replay;

#[cfg(feature = "abi")]
pub use abi::*;
pub use compare::*;
pub use fixture::*;
#[cfg(feature = "mock")]
pub use mock::*;
#[cfg(feature = "mvp-cpu-replay")]
pub use mvp_cpu_replay::*;
pub use outcome::*;
pub use runner::*;

/// Computes a deterministic FNV-1a 64-bit checksum of the dendrite weights plane in a state blob.
pub fn compute_dendrite_weights_checksum(state_blob: &[u8], padded_n: usize) -> u64 {
    let offsets = layout::offsets::compute_state_offsets(padded_n);
    let start = offsets.off_weights;
    let end = offsets.off_dtimers;
    if start >= state_blob.len() || end > state_blob.len() {
        return 0;
    }
    let weights_bytes = &state_blob[start..end];

    // FNV-1a 64-bit hash
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in weights_bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3u64);
    }
    hash
}
