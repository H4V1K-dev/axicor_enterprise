use crate::state::NodeState;
use crate::error::RuntimeError;
use crate::sentinel::Sentinel;
use std::time::Duration;
use std::collections::HashMap;
use wire;

/// Runtime engine coordinating the execution lifecycle of a single node.
pub struct NodeRuntime {
    /// State of the node execution.
    pub state: NodeState,
    
    /// Monotonically increasing tick counter.
    pub tick_counter: u32,
    
    /// Compute shard engine.
    pub shard_engine: compute::ShardEngine,

    /// Health monitor.
    pub sentinel: Sentinel,
    
    /// Sender channel for lock-free command delivery to the hardware driver thread.
    ///
    /// Under INV-RUN-002 (Lock-Free Command Delivery), this is used to push commands
    /// asynchronously to the compute backends without blocking the simulation control loop.
    pub shard_tx: crossbeam::channel::Sender<compute_api::ComputeCommand>,
    
    /// Receiver channel for lock-free command results from the hardware driver thread.
    ///
    /// Under INV-RUN-002 (Lock-Free Command Delivery), this is used to pull batch run
    /// confirmations from the execution thread without mutex locking.
    pub result_rx: crossbeam::channel::Receiver<compute_api::BatchResult>,

    /// Barrier coordinator for cluster causality sync.
    pub bsp_barrier: std::sync::Arc<net::BspBarrier>,

    /// RCU routing table for fast lookup.
    pub routing_table: std::sync::Arc<net::RoutingTable>,

    /// Lock-free state machine wrapper for night phase daemon synchronization.
    pub shm_state: ipc::ShmStateMachine,

    /// Zone hash identifier.
    pub zone_hash: u32,

    /// Raw pointer to the shared memory region.
    pub shm_ptr: *mut u8,

    /// Prune threshold value.
    pub prune_threshold: i16,

    /// Max sprouts per night.
    pub max_sprouts: u16,

    /// Number of simulation ticks before switching to night phase.
    pub night_interval_ticks: u32,

    /// Flag set by signal handlers or administrators to request graceful shutdown.
    pub shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl NodeRuntime {
    /// Create a new NodeRuntime coordinator instance.
    pub fn new(
        shard_engine: compute::ShardEngine,
        shard_tx: crossbeam::channel::Sender<compute_api::ComputeCommand>,
        result_rx: crossbeam::channel::Receiver<compute_api::BatchResult>,
        bsp_barrier: std::sync::Arc<net::BspBarrier>,
        routing_table: std::sync::Arc<net::RoutingTable>,
        shm_state: ipc::ShmStateMachine,
        zone_hash: u32,
        shm_ptr: *mut u8,
        prune_threshold: i16,
        max_sprouts: u16,
        night_interval_ticks: u32,
        shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            state: NodeState::Booting,
            tick_counter: 0,
            shard_engine,
            sentinel: Sentinel::new(),
            shard_tx,
            result_rx,
            bsp_barrier,
            routing_table,
            shm_state,
            zone_hash,
            shm_ptr,
            prune_threshold,
            max_sprouts,
            night_interval_ticks,
            shutdown_flag,
        }
    }

    /// Transition the node state and teardown the compute shard context explicitly.
    pub fn shutdown(mut self) -> Result<(), RuntimeError> {
        // INV-RUN-003: Explicit teardown to prevent C-ABI driver race.
        self.state = NodeState::Shutdown;
        let _ = self.shard_tx.send(compute_api::ComputeCommand::Shutdown);
        self.shard_engine
            .teardown()
            .map_err(RuntimeError::ComputeError)?;
        Ok(())
    }

    /// Run the main runtime state machine orchestration loop.
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        let shutdown_flag = self.shutdown_flag.clone();

        loop {
            if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                self.state = NodeState::Shutdown;
            }

            match self.state {
                NodeState::Booting => {
                    self.state = NodeState::Running;
                }
                NodeState::Running => {
                    if self.tick_counter >= self.night_interval_ticks {
                        self.state = NodeState::Night;
                        continue;
                    }

                    // INV-NET-004: Sync via BSP barrier before running next epoch batch
                    let current_epoch = self.tick_counter / 10; // assuming batch size 10
                    self.bsp_barrier.sync_and_swap(current_epoch)
                        .map_err(|_| RuntimeError::ChannelError)?;

                    self.shard_tx
                        .send(compute_api::ComputeCommand::RunBatch {
                            tick_base: self.tick_counter,
                            batch_size: 10,
                            global_dopamine: 0,
                        })
                        .map_err(|_| RuntimeError::ChannelError)?;

                    let res = self.result_rx.recv().map_err(|_| RuntimeError::ChannelError)?;
                    self.tick_counter += res.ticks_processed;
                }
                NodeState::Night => {
                    self.transition_to_night()?;
                    self.tick_counter = 0;
                    self.state = NodeState::Running;
                }
                NodeState::Resurrection => {
                    self.sentinel.start_warmup();
                    for _ in 0..crate::sentinel::WARMUP_TICKS_LIMIT {
                        self.shard_tx.send(compute_api::ComputeCommand::Resurrect)
                            .map_err(|_| RuntimeError::ChannelError)?;
                        let res = self.result_rx.recv().map_err(|_| RuntimeError::ChannelError)?;
                        // INV-RUN-004: Warmup loop mutes spikes (sentinel verifies stability internally)
                        self.sentinel.verify_stability(&res)?;
                    }
                    self.sentinel.end_warmup();
                    self.state = NodeState::Running;
                }
                NodeState::Shutdown => {
                    let _ = self.shard_tx.send(compute_api::ComputeCommand::Shutdown);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Night phase transition routine coordinating with weaver-daemon.
    fn transition_to_night(&mut self) -> Result<(), RuntimeError> {
        // 1. Prepare SHM state machine for the daemon (switch Idle -> NightStart)
        self.shm_state.prepare_for_daemon().map_err(|_| RuntimeError::ChannelError)?;

        // 2. Read handovers count and extract slices from shared memory region
        let hdr = ipc::validate_shm_header(self.shm_ptr).map_err(|_| RuntimeError::ChannelError)?;
        let (_, _, _, handovers, prunes) = unsafe {
            ipc::extract_slices(self.shm_ptr, hdr)
        };

        // 3. Connect to baker-daemon and trigger night phase sprout logic
        let mut client = ipc::BakerClient::connect(self.zone_hash)
            .map_err(|_| RuntimeError::ChannelError)?;

        let req = wire::BakeRequest {
            magic: *b"BAKE",
            zone_hash: self.zone_hash,
            current_tick: self.tick_counter,
            prune_threshold: self.prune_threshold,
            max_sprouts: self.max_sprouts,
        };

        let acks = client.trigger_night_phase(&req, handovers)
            .map_err(|_| RuntimeError::ChannelError)?;

        // 4. Spin-wait for daemon to transition SHM to NightDone
        self.shm_state.wait_for_daemon(Duration::from_secs(10))
            .map_err(|_| RuntimeError::DaemonTimeout)?;

        // 5. Gather patches (GhostPatch::Add from acks, GhostPatch::Prune from prunes)
        let mut patches = Vec::with_capacity(acks.len() + prunes.len());
        for ack in &acks {
            patches.push(compute_api::GhostPatch::Add {
                src_axon: ack.src_axon_id,
                dst_ghost: ack.dst_ghost_id,
            });
        }
        for prune in prunes {
            patches.push(compute_api::GhostPatch::Prune {
                dst_ghost: prune.dst_ghost_id,
            });
        }

        // 6. Enforce VRAM lock and apply connection updates to accelerator memory
        // INV-RUN-005: Patching and pruning must occur strictly in Night phase
        self.shard_engine.patch_ghosts(&patches).map_err(RuntimeError::ComputeError)?;
        self.shard_engine.run_sort_and_prune(self.prune_threshold).map_err(RuntimeError::ComputeError)?;

        // 7. Perform RCU update on routing table
        let mut new_routes = HashMap::new();
        for ack in &acks {
            // Translate new ghost targets to neighboring network sockets
            if let Some(addr) = self.routing_table.get_address(ack.target_zone_hash) {
                new_routes.insert(ack.target_zone_hash, addr);
            }
        }
        if !new_routes.is_empty() {
            self.routing_table.update_routes(new_routes);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU8};

    struct MockGpu;
    impl compute_api::GpuBackend for MockGpu {
        fn alloc_shard(&self, _layout: &compute_api::ShardLayout) -> Result<compute_api::VramHandle, compute_api::ComputeApiError> {
            Ok(compute_api::VramHandle(1))
        }
        fn upload_state(&self, _handle: &compute_api::VramHandle, _state: &[u8]) -> Result<(), compute_api::ComputeApiError> {
            Ok(())
        }
        fn upload_variants(&self, _handle: &compute_api::VramHandle, _variants: &[layout::VariantParameters]) -> Result<(), compute_api::ComputeApiError> {
            Ok(())
        }
        fn run_day_batch(&self, _handle: &compute_api::VramHandle, _cmd: &compute_api::DayBatchCmd<'_>) -> Result<compute_api::BatchResult, compute_api::ComputeApiError> {
            Ok(compute_api::BatchResult { ticks_processed: 10, is_warmup: false })
        }
        fn download_output(&self, _handle: &compute_api::VramHandle) -> Result<compute_api::OutputFrame, compute_api::ComputeApiError> {
            Ok(compute_api::OutputFrame { data: vec![], num_outputs: 0, sync_batch_ticks: 0 })
        }
        fn download_telemetry(&self, _handle: &compute_api::VramHandle) -> Result<compute_api::TelemetryFrame, compute_api::ComputeApiError> {
            Ok(compute_api::TelemetryFrame { active_soma_ids: vec![], total_spikes: 0 })
        }
        fn patch_ghosts(&self, _handle: &compute_api::VramHandle, _patches: &[compute_api::GhostPatch]) -> Result<(), compute_api::ComputeApiError> {
            Ok(())
        }
        fn run_sort_and_prune(&self, _handle: &compute_api::VramHandle, _prune_threshold: i16) -> Result<(), compute_api::ComputeApiError> {
            Ok(())
        }
        fn free(&self, _handle: compute_api::VramHandle) {}
    }

    fn setup_test_runtime(state: NodeState) -> (NodeRuntime, crossbeam::channel::Receiver<compute_api::ComputeCommand>, crossbeam::channel::Sender<compute_api::BatchResult>) {
        let backend = Box::new(MockGpu);
        let layout = compute_api::ShardLayout { padded_n: 64, total_axons: 100, total_ghosts: 10 };
        let shard_engine = compute::ShardEngine::new(backend, layout).unwrap();

        let (shard_tx, shard_rx_chan) = crossbeam::channel::unbounded();
        let (result_tx_chan, result_rx) = crossbeam::channel::unbounded();

        let bsp_barrier = Arc::new(net::BspBarrier::new(0, 0, transport::WaitStrategy::Eco));
        let routing_table = Arc::new(net::RoutingTable::new());

        let state_val = Arc::new(AtomicU8::new(layout::ShmState::Idle as u8));
        let shm_state = unsafe { ipc::ShmStateMachine::new(Arc::into_raw(state_val) as *const AtomicU8) };

        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let mut runtime = NodeRuntime::new(
            shard_engine,
            shard_tx,
            result_rx,
            bsp_barrier,
            routing_table,
            shm_state,
            12345,
            std::ptr::null_mut(),
            15,
            4,
            100,
            shutdown_flag,
        );
        runtime.state = state;
        (runtime, shard_rx_chan, result_tx_chan)
    }

    #[test]
    fn test_lifecycle_transitions() {
        let (mut runtime, _rx, _tx) = setup_test_runtime(NodeState::Booting);
        assert_eq!(runtime.state, NodeState::Booting);
        
        // Immediate run exit on shutdown flag
        runtime.shutdown_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        let res = runtime.run();
        assert!(res.is_ok());
        assert_eq!(runtime.state, NodeState::Shutdown);
    }

    #[test]
    fn test_crossbeam_channel_isolation() {
        let (runtime, rx, tx) = setup_test_runtime(NodeState::Running);
        assert!(runtime.shard_tx.send(compute_api::ComputeCommand::RunBatch { tick_base: 0, batch_size: 10, global_dopamine: 0 }).is_ok());
        
        let cmd = rx.recv().unwrap();
        match cmd {
            compute_api::ComputeCommand::RunBatch { batch_size, .. } => assert_eq!(batch_size, 10),
            _ => panic!("Expected RunBatch"),
        }

        assert!(tx.send(compute_api::BatchResult { ticks_processed: 10, is_warmup: false }).is_ok());
        let res = runtime.result_rx.recv().unwrap();
        assert_eq!(res.ticks_processed, 10);
    }

    #[test]
    fn test_warmup_loop_mute() {
        // In Resurrection, NodeRuntime runs 100 warmup ticks
        let (mut runtime, rx, tx) = setup_test_runtime(NodeState::Resurrection);

        let shutdown = runtime.shutdown_flag.clone();
        let handle = std::thread::spawn(move || {
            for _ in 0..crate::sentinel::WARMUP_TICKS_LIMIT {
                let cmd = rx.recv().unwrap();
                assert_eq!(cmd, compute_api::ComputeCommand::Resurrect);
                tx.send(compute_api::BatchResult { ticks_processed: 1, is_warmup: true }).unwrap();
            }
            // Trigger shutdown after warmup to exit run loop
            shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let res = runtime.run();
        assert!(res.is_ok());
        handle.join().unwrap();
        assert_eq!(runtime.sentinel.warmup_ticks, 0);
    }
}
