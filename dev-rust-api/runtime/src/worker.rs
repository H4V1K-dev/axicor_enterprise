use compute_api::{ComputeCommand, BatchResult, DayBatchCmd};

/// Background worker thread execution routine for the accelerator.
///
/// ### Invariants
///
/// - **INV-RUN-001: Thread Isolation**
///   The hardware interaction with the GPU/CPU compute backend must occur exclusively on this
///   dedicated background worker thread to ensure MESI cache locality and thread safety.
///
/// - **INV-RUN-002: Lock-Free Command Delivery**
///   Orchestration commands and execution results are passed asynchronously between the control
///   thread and the worker thread using lock-free channels to prevent hot path blocking.
pub fn run_shard_thread(
    engine: &compute::ShardEngine,
    rx: &crossbeam::channel::Receiver<ComputeCommand>,
    tx: &crossbeam::channel::Sender<BatchResult>,
) {
    while let Ok(cmd) = rx.recv() {
        match cmd {
            ComputeCommand::RunBatch {
                tick_base,
                batch_size,
                global_dopamine,
            } => {
                let spike_counts = vec![0u32; batch_size as usize];
                let mapped_somas = vec![];
                let cmd = DayBatchCmd {
                    tick_base,
                    sync_batch_ticks: batch_size,
                    v_seg: 0,
                    global_dopamine,
                    virtual_offset: 0,
                    num_virtual_axons: 0,
                    num_outputs: 0,
                    input_bitmask: None,
                    incoming_spikes: None,
                    spike_counts: &spike_counts,
                    mapped_soma_ids: &mapped_somas,
                    ephys_cmd: None,
                };
                if let Ok(res) = engine.run_day_batch(&cmd) {
                    let _ = tx.send(res);
                }
            }
            ComputeCommand::Resurrect => {
                let mock_result = BatchResult {
                    ticks_processed: 0,
                    is_warmup: true,
                };
                let _ = tx.send(mock_result);
            }
            ComputeCommand::Shutdown => {
                break;
            }
        }
    }
}
