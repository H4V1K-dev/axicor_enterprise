//! Deterministic fixture construction for conformance and differential testing.

use compute_api::{DayBatchCmd, ShardAllocSpec, ShardUpload};
use layout::{VariantParameters, VARIANT_LUT_LEN};
use types::AXON_SENTINEL;

/// A reusable, deterministic test fixture for Layer 3 backends.
pub struct ConformanceFixture {
    /// Name of the test fixture.
    pub name: String,
    /// Memory allocation specification parameters.
    pub spec: ShardAllocSpec,
    /// Neuron profile parameters.
    pub variant_table: [VariantParameters; VARIANT_LUT_LEN],
    /// Binary state blob matching the specification.
    pub state_blob: Vec<u8>,
    /// Binary axons table blob matching the specification.
    pub axons_blob: Vec<u8>,
}

/// Owned buffer storage for day batch command execution.
pub struct FixtureCmdBuffers {
    /// Input bitmask payload.
    pub input_bitmask: Vec<u32>,
    /// Incoming spike ID payload.
    pub incoming_spikes: Vec<u32>,
    /// Per-tick incoming spike counts.
    pub incoming_spike_counts: Vec<u32>,
    /// Soma indices mapped to output monitors.
    pub mapped_soma_ids: Vec<u32>,
    /// Target buffer for generated output spike IDs.
    pub output_spikes: Vec<u32>,
    /// Target buffer for generated per-tick output spike counts.
    pub output_spike_counts: Vec<u32>,
}

impl ConformanceFixture {
    /// Constructs a new deterministic fixture.
    pub fn new(
        name: &str,
        padded_n: u32,
        total_axons: u32,
        total_ghosts: u32,
        virtual_offset: u32,
    ) -> Self {
        let spec = ShardAllocSpec {
            padded_n,
            total_axons,
            total_ghosts,
            virtual_offset,
        };

        // Initialize variant table with deterministic data
        let dummy = VariantParameters {
            threshold: 0,
            rest_potential: 0,
            leak_shift: 0,
            homeostasis_penalty: 0,
            spontaneous_firing_period_ticks: 0,
            initial_synapse_weight: 0,
            gsop_potentiation: 0,
            gsop_depression: 0,
            homeostasis_decay: 0,
            refractory_period: 0,
            synapse_refractory_period: 0,
            signal_propagation_length: 0,
            is_inhibitory: 0,
            inertia_curve: [0; 8],
            ahp_amplitude: 0,
            _pad1: [0; 6],
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
            _leak_pad: [0; 3],
            d1_affinity: 0,
            d2_affinity: 0,
            heartbeat_m: 0,
        };

        let mut variant_table = [dummy; VARIANT_LUT_LEN];
        for (i, var) in variant_table.iter_mut().enumerate() {
            *var = VariantParameters {
                threshold: -50000,
                rest_potential: -70000,
                leak_shift: 1,
                homeostasis_penalty: 1000,
                spontaneous_firing_period_ticks: 100,
                initial_synapse_weight: (100 + i) as u16,
                gsop_potentiation: 10,
                gsop_depression: 5,
                homeostasis_decay: 10,
                refractory_period: 2,
                synapse_refractory_period: 1,
                signal_propagation_length: 5,
                is_inhibitory: if i % 4 == 0 { 1 } else { 0 },
                inertia_curve: [0; 8],
                ahp_amplitude: 500,
                _pad1: [0; 6],
                adaptive_leak_min_shift: 2,
                adaptive_leak_gain: 10,
                adaptive_mode: 0,
                _leak_pad: [0; 3],
                d1_affinity: 10,
                d2_affinity: 5,
                heartbeat_m: 65535,
            };
        }

        // Calculate and build state blob with correct headers and sizes (written manually)
        let state_size = layout::calculate_state_blob_size(padded_n as usize);
        let mut state_blob = vec![0u8; state_size];
        state_blob[0..4].copy_from_slice(&layout::STATE_MAGIC);
        state_blob[4..8].copy_from_slice(&layout::STATE_FILE_VERSION.to_le_bytes());
        state_blob[8..12].copy_from_slice(&padded_n.to_le_bytes());
        state_blob[12..16].copy_from_slice(&total_axons.to_le_bytes());

        // Calculate and build axons blob with correct headers, sizes, and sentinels
        let axons_size = compute_api::validation::expected_axons_blob_size(total_axons)
            .unwrap_or(16 + total_axons as usize * 32);
        let mut axons_blob = vec![0u8; axons_size];
        axons_blob[0..4].copy_from_slice(&layout::AXONS_MAGIC);
        axons_blob[4..8].copy_from_slice(&layout::AXONS_FILE_VERSION.to_le_bytes());
        axons_blob[8..12].copy_from_slice(&total_axons.to_le_bytes());
        axons_blob[12..16].copy_from_slice(&0u32.to_le_bytes());

        // Initialize BurstHeads8 manually in axons_blob with 8 little-endian u32 values of AXON_SENTINEL
        let mut empty_burst = [0u8; 32];
        let sentinel_bytes = AXON_SENTINEL.to_le_bytes();
        for chunk in empty_burst.chunks_exact_mut(4) {
            chunk.copy_from_slice(&sentinel_bytes);
        }

        let head_size = 32;
        for i in 0..total_axons as usize {
            let offset = 16 + i * head_size;
            axons_blob[offset..offset + head_size].copy_from_slice(&empty_burst);
        }

        Self {
            name: String::from(name),
            spec,
            variant_table,
            state_blob,
            axons_blob,
        }
    }

    /// Assembles a `ShardUpload` structure borrowing the fixture blobs.
    pub fn upload(&self) -> ShardUpload<'_> {
        ShardUpload {
            state_blob: &self.state_blob,
            axons_blob: &self.axons_blob,
            variant_table: &self.variant_table,
        }
    }

    /// Creates command buffer storage initialized with deterministic values.
    pub fn create_cmd_buffers(
        &self,
        ticks: u32,
        max_spikes: u32,
        input_words: u32,
        num_outputs: u32,
    ) -> FixtureCmdBuffers {
        let input_bitmask = vec![0xAAAAAAAAu32; (input_words * ticks) as usize];
        let mut incoming_spikes = vec![0u32; (max_spikes * ticks) as usize];
        let mut incoming_spike_counts = vec![0u32; ticks as usize];

        // Place a few deterministic incoming spikes
        for t in 0..ticks as usize {
            if max_spikes > 0 {
                incoming_spike_counts[t] = 1;
                incoming_spikes[t * max_spikes as usize] =
                    self.spec.virtual_offset + (t as u32 % 100);
            }
        }

        let mapped_soma_ids = (0..num_outputs)
            .map(|i| i % self.spec.padded_n)
            .collect::<Vec<_>>();

        let output_spikes = vec![0u32; (max_spikes * ticks) as usize];
        let output_spike_counts = vec![0u32; ticks as usize];

        FixtureCmdBuffers {
            input_bitmask,
            incoming_spikes,
            incoming_spike_counts,
            mapped_soma_ids,
            output_spikes,
            output_spike_counts,
        }
    }

    /// Creates a borrowed `DayBatchCmd` payload using the provided buffers.
    #[allow(clippy::too_many_arguments)]
    pub fn build_cmd<'a>(
        &self,
        tick_base: u64,
        ticks: u32,
        v_seg: u32,
        dopamine: i16,
        input_words: u32,
        max_spikes: u32,
        num_outputs: u32,
        num_virtual_axons: u32,
        bufs: &'a mut FixtureCmdBuffers,
    ) -> DayBatchCmd<'a> {
        DayBatchCmd {
            tick_base,
            sync_batch_ticks: ticks,
            v_seg,
            dopamine,
            input_words_per_tick: input_words,
            max_spikes_per_tick: max_spikes,
            num_outputs,
            virtual_offset: self.spec.virtual_offset,
            num_virtual_axons,
            input_bitmask: Some(&bufs.input_bitmask),
            incoming_spikes: Some(&bufs.incoming_spikes),
            incoming_spike_counts: &bufs.incoming_spike_counts,
            mapped_soma_ids: &bufs.mapped_soma_ids,
            output_spikes: &mut bufs.output_spikes,
            output_spike_counts: &mut bufs.output_spike_counts,
        }
    }
}
