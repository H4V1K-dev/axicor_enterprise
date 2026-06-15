use crate::state::NodeState;
use crate::error::RuntimeError;

const NIGHT_INTERVAL_TICKS: u32 = 10000;

/// Runtime engine coordinating the execution lifecycle of a single node.
pub struct NodeRuntime {
    /// State of the node execution.
    pub state: NodeState,
    
    /// Monotonically increasing tick counter.
    pub tick_counter: u32,
    
    /// Compute shard engine.
    pub shard_engine: compute::ShardEngine,
    
    /// Sender channel for lock-free command delivery to the hardware driver thread.
    ///
    /// Under INV-RUN-002 (Lock-Free Command Delivery), this is used to push commands
    /// asynchronously to the compute backends without blocking the simulation control loop.
    pub shard_tx: crossbeam::channel::Sender<compute_api::ComputeCommand>,
    
    /// Receiver channel for the background thread to receive commands.
    pub shard_rx: crossbeam::channel::Receiver<compute_api::ComputeCommand>,

    /// Sender channel for the background thread to send batch results.
    pub result_tx: crossbeam::channel::Sender<compute_api::BatchResult>,

    /// Receiver channel for lock-free command results from the hardware driver thread.
    ///
    /// Under INV-RUN-002 (Lock-Free Command Delivery), this is used to pull batch run
    /// confirmations from the execution thread without mutex locking.
    pub result_rx: crossbeam::channel::Receiver<compute_api::BatchResult>,

    /// Flag set by signal handlers or administrators to request graceful shutdown.
    pub shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl NodeRuntime {
    /// Transition the node state and teardown the compute shard context explicitly.
    pub fn shutdown(mut self) -> Result<(), RuntimeError> {
        // INV-RUN-003: Explicit teardown to prevent C-ABI driver race.
        self.state = NodeState::Shutdown;
        self.shard_engine
            .teardown()
            .map_err(RuntimeError::ComputeError)?;
        Ok(())
    }

    /// Run the main runtime state machine orchestration loop.
    pub fn run(&mut self) -> Result<(), RuntimeError> {
        let engine = &self.shard_engine;
        let rx = &self.shard_rx;
        let tx = &self.result_tx;

        let state = &mut self.state;
        let tick_counter = &mut self.tick_counter;
        let shard_tx = &self.shard_tx;
        let result_rx = &self.result_rx;
        let shutdown_flag = &self.shutdown_flag;

        std::thread::scope(|s| {
            s.spawn(|| {
                crate::worker::run_shard_thread(engine, rx, tx);
            });

            loop {
                if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    *state = NodeState::Shutdown;
                }

                match *state {
                    NodeState::Booting => {
                        *state = NodeState::Running;
                    }
                    NodeState::Running => {
                        if *tick_counter >= NIGHT_INTERVAL_TICKS {
                            *state = NodeState::Night;
                            continue;
                        }
                        shard_tx
                            .send(compute_api::ComputeCommand::RunBatch {
                                tick_base: *tick_counter,
                                batch_size: 10,
                                global_dopamine: 0,
                            })
                            .map_err(|_| RuntimeError::ChannelError)?;

                        let _res = result_rx.recv().map_err(|_| RuntimeError::ChannelError)?;
                        *tick_counter += 10;
                    }
                    NodeState::Night => {
                        *tick_counter = 0;
                        *state = NodeState::Running;
                    }
                    NodeState::Resurrection => {
                        *state = NodeState::Running;
                    }
                    NodeState::Shutdown => {
                        let _ = shard_tx.send(compute_api::ComputeCommand::Shutdown);
                        break;
                    }
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_runtime_shutdown_method() {
        let backend = compute::instantiate_backend(compute::BackendType::Cpu, None).unwrap();
        let layout = compute_api::ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 10,
        };
        let shard_engine = compute::ShardEngine::new(backend, layout).unwrap();
        let (shard_tx, shard_rx) = crossbeam::channel::unbounded();
        let (result_tx, result_rx) = crossbeam::channel::unbounded();

        let runtime = NodeRuntime {
            state: NodeState::Booting,
            tick_counter: 0,
            shard_engine,
            shard_tx,
            shard_rx,
            result_tx,
            result_rx,
            shutdown_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        assert!(!runtime.shard_engine.is_teared_down());
        let res = runtime.shutdown();
        assert!(res.is_ok());
    }

    #[test]
    fn test_node_runtime_immediate_shutdown_run() {
        let backend = compute::instantiate_backend(compute::BackendType::Cpu, None).unwrap();
        let layout = compute_api::ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 10,
        };
        let shard_engine = compute::ShardEngine::new(backend, layout).unwrap();
        let (shard_tx, shard_rx) = crossbeam::channel::unbounded();
        let (result_tx, result_rx) = crossbeam::channel::unbounded();

        let mut runtime = NodeRuntime {
            state: NodeState::Shutdown,
            tick_counter: 0,
            shard_engine,
            shard_tx,
            shard_rx,
            result_tx,
            result_rx,
            shutdown_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        let res = runtime.run();
        assert!(res.is_ok());
        assert_eq!(runtime.state, NodeState::Shutdown);
    }
}
