//! Data Transfer Objects (DTOs) for simulation memory allocation, data upload, batch commands, and telemetry results.

/// Specification parameters required for VRAM allocation of a simulation shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardAllocSpec {
    /// Aligned soma allocation count (`padded_n`). Must be non-zero and aligned to 64.
    pub padded_n: u32,
    /// Total number of active axon instances in the shard.
    pub total_axons: u32,
    /// Total number of ghost axon slots in inter-shard communication planes.
    pub total_ghosts: u32,
    /// Global virtual axon ID offset for this shard.
    pub virtual_offset: u32,
}

/// Borrowed host byte slices containing binary state and axon structures for initial VRAM upload.
#[derive(Debug)]
pub struct ShardUpload<'a> {
    /// Per-plane 64B aligned `.state` dump binary blob.
    pub state_blob: &'a [u8],
    /// Axon burst table `.axons` binary blob.
    pub axons_blob: &'a [u8],
    /// Borrowed neuron variant parameter profile table (16 entries).
    pub variant_table: &'a [layout::VariantParameters; layout::VARIANT_LUT_LEN],
}

/// Execution command payload containing inputs, outputs, and parameters for a day batch run.
#[derive(Debug)]
pub struct DayBatchCmd<'a> {
    /// Base absolute tick counter at the start of this day batch.
    pub tick_base: u64,
    /// Number of simulation ticks to execute synchronously in this batch.
    pub sync_batch_ticks: u32,
    /// Conduction velocity segmentation factor (1..=255).
    pub v_seg: u32,
    /// Global neuromodulatory dopamine level for STDP weight updates.
    pub dopamine: i16,
    /// Number of 32-bit words per tick in the input bitmask.
    pub input_words_per_tick: u32,
    /// Maximum number of incoming/outgoing spikes capacity per tick.
    pub max_spikes_per_tick: u32,
    /// Number of output soma monitors configured.
    pub num_outputs: u32,
    /// Global virtual axon ID offset.
    pub virtual_offset: u32,
    /// Number of virtual axons mapped to this shard.
    pub num_virtual_axons: u32,
    /// Optional flattened input bitmask slice (`input_words_per_tick * sync_batch_ticks`).
    pub input_bitmask: Option<&'a [u32]>,
    /// Optional flattened incoming spike ID payload slice (`sync_batch_ticks * max_spikes_per_tick`).
    pub incoming_spikes: Option<&'a [u32]>,
    /// Per-tick incoming spike count slice (`sync_batch_ticks`).
    pub incoming_spike_counts: &'a [u32],
    /// Soma indices mapped to output monitors (`num_outputs`).
    pub mapped_soma_ids: &'a [u32],
    /// Target buffer slice for generated output spike IDs (`sync_batch_ticks * max_spikes_per_tick`).
    pub output_spikes: &'a mut [u32],
    /// Target buffer slice for generated per-tick output spike counts (`sync_batch_ticks`).
    pub output_spike_counts: &'a mut [u32],
}

/// Summary statistics and telemetry results returned after completing a day batch execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchResult {
    /// Total number of simulation ticks executed.
    pub ticks_executed: u32,
    /// Total number of internal spike firing events generated.
    pub generated_spikes_count: u32,
    /// Total number of output spikes written to `output_spikes`.
    pub output_spikes_written: u32,
    /// Number of generated spikes dropped due to buffer capacity limits.
    pub dropped_spikes_count: u32,
    /// Execution duration in microseconds.
    pub execution_time_us: u64,
}

/// Mutable host byte slices for extracting full-state VRAM diagnostics snapshots.
#[derive(Debug)]
pub struct ShardSnapshotMut<'a> {
    /// Target mutable slice for the `.state` dump blob.
    pub state_blob: &'a mut [u8],
    /// Target mutable slice for the `.axons` dump blob.
    pub axons_blob: &'a mut [u8],
}
