//! C-ABI layout definition for neuron profile variant parameters.

use bytemuck::{Pod, Zeroable};

/// Biological and physical execution parameters defining a neuron type profile.
///
/// L1/L2 cache-line aligned (64 bytes) POD structure for GPU Constant Memory resident tables.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct VariantParameters {
    /// Base spike threshold potential.
    pub threshold: i32,
    /// Resting membrane potential.
    pub rest_potential: i32,
    /// Bit shift for exponential leak calculation.
    pub leak_shift: u32,
    /// Threshold increment penalty applied upon firing.
    pub homeostasis_penalty: i32,
    /// Spontaneous firing period in ticks (DDS).
    pub spontaneous_firing_period_ticks: u32,
    /// Initial weight assigned to newly established synapses.
    pub initial_synapse_weight: u16,
    /// Base GSOP potentiation impulse magnitude.
    pub gsop_potentiation: u16,
    /// Base GSOP depression impulse magnitude.
    pub gsop_depression: u16,
    /// Homeostasis decay scaling factor.
    pub homeostasis_decay: u16,
    /// Soma refractory period duration in ticks.
    pub refractory_period: u8,
    /// Maximum fatigue capacity for synaptic gradient fatigue in ticks (1..=255).
    pub fatigue_capacity: u8,
    /// Signal propagation active tail length.
    pub signal_propagation_length: u8,
    /// Inhibitory flag (1 for inhibitory / GABA, 0 for excitatory / Glu).
    pub is_inhibitory: u8,
    /// Lookup table for GSOP inertia curve coefficients.
    pub inertia_curve: [u8; 8],
    /// After-hyperpolarization (AHP) trace amplitude.
    pub ahp_amplitude: u16,
    /// Explicit padding bytes to align to 48-byte boundary.
    pub _pad1: [u8; 6],
    /// Minimum bit shift for adaptive leak mechanism.
    pub adaptive_leak_min_shift: i32,
    /// Amplification gain for adaptive leak.
    pub adaptive_leak_gain: u16,
    /// Operating mode selector for adaptive leak.
    pub adaptive_mode: u8,
    /// Explicit padding bytes to align to 58-byte boundary.
    pub _leak_pad: [u8; 3],
    /// Affinity coefficient for D1 dopamine receptors.
    pub d1_affinity: u8,
    /// Affinity coefficient for D2 dopamine receptors.
    pub d2_affinity: u8,
    /// Precalculated phase step multiplier for DDS heartbeat.
    pub heartbeat_m: u32,
}
