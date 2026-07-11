//! DTO definitions for the runtime coordinator.

/// Configuration parameters for a single-shard local day loop runtime orchestration.
#[derive(Debug, Clone)]
pub struct LocalRuntimeConfig {
    /// Number of simulation ticks executed per batch.
    pub sync_batch_ticks: u32,
    /// Conduction delay segmentation steps per biological axon.
    pub v_seg: u32,
    /// Global neuromodulatory dopamine level for STDP updates.
    pub dopamine: i16,
    /// Maximum capacity limits for input/output spikes per tick.
    pub max_spikes_per_tick: u32,
    /// Global virtual axon ID offset.
    pub virtual_offset: u32,
    /// Number of virtual axons mapped.
    pub num_virtual_axons: u32,
    /// Words size per tick in input bitmask arrays.
    pub input_words_per_tick: u32,
    /// Soma indices mapped to output monitoring targets.
    pub mapped_soma_ids: Vec<u32>,
}

/// Lifecycle states of the local runtime orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    /// Initialized but not yet running simulation cycles.
    Created,
    /// Ready and executing day batches.
    Running,
    /// Gracefully shutdown and underlying resources freed.
    Stopped,
    /// Terminated due to unrecoverable compute execution faults.
    Faulted,
}

/// Cumulative telemetry metrics recorded by the local runtime.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RuntimeStats {
    /// Current biological tick count.
    pub current_tick: u64,
    /// Total count of successfully coordinated execution batches.
    pub batches_executed: u64,
    /// Cumulative count of biological ticks simulated.
    pub ticks_executed: u64,
    /// Cumulative sum of internal spikes generated across all simulated steps.
    pub generated_spikes: u64,
    /// Cumulative sum of output spikes written to buffers.
    pub output_spikes_written: u64,
    /// Cumulative sum of output spikes dropped due to capacity limits.
    pub dropped_spikes: u64,
    /// Total count of compute failures encountered.
    pub compute_errors: u64,
}

/// Borrowed payload of incoming spike buffers supplied to a day batch.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeBatchInput<'a> {
    /// Optional flattened array of input channel masks.
    pub input_bitmask: Option<&'a [u32]>,
    /// Optional flattened indices of incoming input spikes.
    pub incoming_spikes: Option<&'a [u32]>,
    /// Per-tick counts of input spikes. Length must equal `sync_batch_ticks`.
    pub incoming_spike_counts: &'a [u32],
}

/// Summary report returned upon successful batch step completion.
#[derive(Debug, Clone)]
pub struct RuntimeBatchReport {
    /// Nizko-level telemetry results returned by the compute backend.
    pub batch_result: compute_api::BatchResult,
    /// Copy of output spike indices generated during this batch run.
    pub output_spikes: Vec<u32>,
    /// Copy of per-tick output spike count headers.
    pub output_spike_counts: Vec<u32>,
    /// Start biological tick index of the calculated batch.
    pub tick_base: u64,
    /// Total ticks executed in this step.
    pub ticks_executed: u32,
}

/// Durable host state copy and coordinates storage for Day/Night orchestration.
#[derive(Debug, Clone)]
pub struct HostWorkingState {
    /// Durable host copy of somatic and dendritic state blob (export/import target).
    pub state_blob: Vec<u8>,
    /// Durable host copy of axons burst heads blob (export/import target).
    pub axons_blob: Vec<u8>,
    /// Durable axon paths coordinate coordinates list. NEVER zero-wiped for "fresh night".
    pub paths_blob: Vec<u8>,
    /// Count of aligned soma neurons.
    pub padded_n: u32,
    /// Total count of active axons.
    pub total_axons: u32,
    /// Total count of ghost axons.
    pub total_ghosts: u32,
}

/// Night phase execution parameters.
#[derive(Debug, Clone)]
pub struct NightJobParams {
    /// Unique shard identification index.
    pub shard_id: u32,
    /// Unique configuration/layout identifier hash.
    pub zone_hash: u32,
    /// Epoch identifier index for this night.
    pub night_epoch: u64,
    /// Seed bytes for stochastic operations.
    pub master_seed: [u8; 32],
    /// Pruning threshold (in Mass Domain, i32, checked >= 0 and converted to u32).
    pub prune_threshold: i32,
    /// Maximum sprout count permitted.
    pub max_sprouts: u32,
    /// Maximum synaptic growth distance.
    pub w_distance: u32,
    /// Scaling parameter for distance cost.
    pub w_power: u32,
    /// Scaling parameter for exploration noise.
    pub w_explore: u32,
    /// Initial synaptic weight value.
    pub initial_synapse_weight: i32,
}

