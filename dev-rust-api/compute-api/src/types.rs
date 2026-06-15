/// Reserved invalid/uninitialized VRAM handle value to protect against Use-After-Free.
pub const INVALID_VRAM_HANDLE: u64 = 0;

/// Hardware limit for concurrent target recording/stimulation targets inside EphysCmd.
pub const MAX_EPHYS_TARGETS: u32 = 16;

/// Opaque wrapper around a u64 identifying an allocated memory region in VRAM.
///
/// Under INV-COMPUTE-API-002, this wrapper isolates the host from raw GPU pointers.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VramHandle(pub u64);

/// Geometrical metadata structure for accelerator memory allocation.
///
/// Controls the dimensions of the Flat Allocation blocks in VRAM/RAM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardLayout {
    pub padded_n: u32,
    pub total_axons: u32,
    pub total_ghosts: u32,
}

/// Payload describing parameters to run a hot simulation epoch on the accelerator.
///
/// Under INV-COMPUTE-API-004, the lifetime `'a` binds the references to Pinned RAM
/// to prevent Use-After-Free during asynchronous DMA copy operations.
pub struct DayBatchCmd<'a> {
    pub tick_base: u32,
    pub sync_batch_ticks: u32,
    pub v_seg: u32,
    pub global_dopamine: i16,
    pub virtual_offset: u32,
    pub num_virtual_axons: u32,
    pub num_outputs: u32,
    pub input_bitmask: Option<&'a [u8]>,
    pub incoming_spikes: Option<&'a [u8]>,
    pub spike_counts: &'a [u32],
    pub mapped_soma_ids: &'a [u32],
    pub ephys_cmd: Option<EphysCmd>,
}

unsafe impl<'a> Send for DayBatchCmd<'a> {}
unsafe impl<'a> Sync for DayBatchCmd<'a> {}

/// Confirmation payload of a completed HFT-cycle batch run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchResult {
    pub ticks_processed: u32,
    pub is_warmup: bool,
}

/// Readout payload of motor commands extracted from accelerator memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFrame {
    pub data: Vec<u8>,
    pub num_outputs: u32,
    pub sync_batch_ticks: u32,
}

/// Activity telemetry payload containing recorded spikes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryFrame {
    pub active_soma_ids: Vec<u32>,
    pub total_spikes: u32,
}

/// Patch command to mutate inter-shard connections inside VRAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhostPatch {
    /// O(1) insertion of a new route at the end of routing arrays.
    Add { src_axon: u32, dst_ghost: u32 },
    /// O(1) pruning of a route via Swap-and-Pop.
    Prune { dst_ghost: u32 },
}

/// Management control payload for the reserved inter-zone routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicCapacityRouting {
    pub capacity: u32,
    pub active_routes: u32,
}

/// Controlling command sent to the Shard Thread to orchestrate the lifecycle of the accelerator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeCommand {
    /// Run a simulation epoch on the given batch size.
    RunBatch {
        tick_base: u32,
        batch_size: u32,
        global_dopamine: i16,
    },
    /// Prepare the device memory for resurrection and potential warmup.
    Resurrect,
    /// Stop execution and cleanly free C-ABI resources.
    Shutdown,
}

/// Electrophysiology debug command containing device pointers.
///
/// Implements `Send` and `Sync` to allow safe transfer of execution contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EphysCmd {
    pub tids_d: *const u32,
    pub uvs_d: *const i32,
    pub trace_d: *mut i32,
    pub count: u32,
    pub max_ticks: u32,
    pub current_tick: u32,
}

unsafe impl Send for EphysCmd {}
unsafe impl Sync for EphysCmd {}
