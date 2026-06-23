use clap::Parser;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Parser)]
#[command(
    name = "axicor-node",
    about = "Distributed Axicor Brain Node Daemon",
    version
)]
struct Cli {
    /// Path to .axic archive
    #[arg(short, long)]
    archive: std::path::PathBuf,
    
    /// Specific zone to launch (if not specified, default to first found)
    #[arg(short, long)]
    zone: Option<String>,
    
    /// Compute backend type (cpu, cuda, hip)
    #[arg(short, long, default_value = "cpu")]
    backend: String,
}

/// Simulation node daemon entry point.
///
/// ### Invariants
///
/// - **INV-NODE-001: Graceful Shutdown**
///   The node daemon registers a Ctrl-C signal handler that sets a thread-safe atomic
///   `shutdown_flag` to notify the state machine loop of incoming termination requests.
///
/// - **INV-CROSS-011: Node Teardown Sync**
///   Upon completion of the HFT loop, the node main thread blocks to explicitly clean up GPU
///   VRAM state handles before exiting, eliminating race conditions with OS drivers during shutdown.
fn main() {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    
    // INV-NODE-001: Register Ctrl-C handler for graceful shutdown
    ctrlc::set_handler({
        let flag = shutdown_flag.clone();
        move || {
            flag.store(true, Ordering::SeqCst);
            tracing::warn!("SIGINT/SIGTERM received. Initiating Graceful Shutdown...");
        }
    })
    .expect("Error setting Ctrl-C handler");
    
    // Determine backend type
    let backend_type = match cli.backend.as_str() {
        "cuda" => compute::BackendType::Cuda,
        "hip" => compute::BackendType::Hip,
        "cpu" => compute::BackendType::Cpu,
        _ => {
            tracing::error!("Unknown backend type: {}", cli.backend);
            std::process::exit(1);
        }
    };

    // Phase 1 to 8: Execute Boot Pipeline
    let (shard_engine, manifest, tmpfs_dir) = boot::BootPipeline::execute(
        &cli.archive,
        cli.zone.as_deref(),
        backend_type,
    ).unwrap_or_else(|e| {
        tracing::error!("Boot failed: {}", e);
        std::process::exit(1);
    });

    tracing::info!(
        zone_hash = manifest.zone_hash,
        "Boot pipeline completed successfully. Active zone initialized."
    );

    // Initialize OS shared memory block for weaver-daemon coordination
    let shm_manager = ipc::ShmManager::create_cold(manifest.zone_hash, manifest.memory.padded_n)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to initialize shared memory: {:?}", e);
            std::process::exit(1);
        });

    let shm_ptr = shm_manager.mapped.mmap.as_ptr() as *mut u8;
    let state_ptr = unsafe {
        &*(std::ptr::addr_of!((*shm_ptr.cast::<layout::ShmHeader>()).state)
            as *const std::sync::atomic::AtomicU8)
    };
    let shm_state = unsafe { ipc::ShmStateMachine::new(state_ptr) };

    // Set up network barrier and routing table
    let bsp_barrier = Arc::new(net::BspBarrier::new(
        0,
        manifest.network.fast_path_peers.len(),
        transport::WaitStrategy::Eco,
    ));

    let routing_table = Arc::new(net::RoutingTable::new());
    let mut initial_routes = std::collections::HashMap::new();
    for (peer_name, peer_addr_str) in &manifest.network.fast_path_peers {
        if let Ok(addr) = peer_addr_str.parse() {
            let peer_hash = types::fnv1a_32(peer_name.as_bytes());
            initial_routes.insert(peer_hash, addr);
        }
    }
    routing_table.update_routes(initial_routes);

    // Set up lock-free command queues for thread communication
    let (shard_tx, shard_rx) = crossbeam::channel::unbounded();
    let (result_tx, result_rx) = crossbeam::channel::unbounded();
    
    let mut runtime = runtime::NodeRuntime::new(
        shard_engine,
        shard_tx,
        result_rx,
        bsp_barrier,
        routing_table,
        shm_state,
        manifest.zone_hash,
        shm_ptr,
        manifest.settings.plasticity.prune_threshold,
        manifest.settings.plasticity.max_sprouts,
        manifest.settings.night_interval_ticks as u32,
        shutdown_flag,
    );
    
    // Spawn OS execution context thread and run orchestrator loop
    let shard_engine_ptr = &runtime.shard_engine as *const compute::ShardEngine as usize;
    std::thread::scope(|s| {
        // Spawn compute worker thread
        let shard_rx_chan = shard_rx.clone();
        let result_tx_chan = result_tx.clone();
        s.spawn(move || {
            // SAFETY: The scoped thread is joined before runtime is dropped (at the end of std::thread::scope).
            // No concurrent mutating operations occur on the shard engine since the worker only executes during the Day phase.
            let shard_engine_ref = unsafe { &*(shard_engine_ptr as *const compute::ShardEngine) };

            // Apply CPU affinity core lock strictly on Linux to prevent scheduler thrashing
            #[cfg(target_os = "linux")]
            {
                let mut cpuset: libc::cpu_set_t = unsafe { std::mem::zeroed() };
                unsafe { libc::CPU_SET(0, &mut cpuset) };
                let res = unsafe {
                    libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &cpuset)
                };
                if res == 0 {
                    tracing::info!("Compute execution thread locked to physical CPU Core 0");
                }
            }

            runtime::worker::run_shard_thread(shard_engine_ref, &shard_rx_chan, &result_tx_chan);
        });

        // Run control loop orchestrator
        runtime.run().unwrap_or_else(|e| {
            tracing::error!("Runtime execution loop failure: {}", e);
        });
    });
    
    // INV-CROSS-011: Block thread to join and cleanly drop hardware bindings
    runtime.shutdown().expect("Node context teardown failed");
    
    // Clean up OS tmpfs artifacts explicitly on normal termination
    drop(tmpfs_dir);

    tracing::info!("Node daemon terminated gracefully.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_argument_parsing() {
        let args = vec![
            "axicor-node",
            "--archive",
            "path/to/model.axic",
            "--backend",
            "cuda",
            "--zone",
            "SensoryCortex",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.archive, std::path::PathBuf::from("path/to/model.axic"));
        assert_eq!(cli.backend, "cuda");
        assert_eq!(cli.zone, Some("SensoryCortex".to_string()));
    }
}
