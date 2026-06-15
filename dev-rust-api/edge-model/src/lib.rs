pub mod error;
pub mod distill;
pub mod export;

/// Configuration for the Edge Model Compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeConfig {
    /// Target number of dendrite slots (K) per neuron.
    /// Values must be in the range [1, 128].
    pub target_dendrite_slots: usize,
}

/// Distilled hardware-compatible model sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeModel {
    /// SRAM section containing hot, mutable runtime states (voltage, timers, heads).
    pub sram_blob: Vec<u8>,
    /// Flash section containing read-only topology and weights, padded to 64KB.
    pub flash_blob: Vec<u8>,
}
