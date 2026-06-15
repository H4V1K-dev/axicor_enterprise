use clap::Parser;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    archive: std::path::PathBuf,
    
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
    
    ctrlc::set_handler({
        let flag = shutdown_flag.clone();
        move || {
            flag.store(true, Ordering::SeqCst);
            tracing::warn!("SIGINT received. Initiating Graceful Shutdown...");
        }
    })
    .expect("Error setting Ctrl-C handler");
    
    let shard_engine = boot::BootPipeline::execute(&cli.archive).unwrap_or_else(|e| {
        tracing::error!("Boot failed: {}", e);
        std::process::exit(1);
    });
    
    let (shard_tx, shard_rx) = crossbeam::channel::unbounded();
    let (result_tx, result_rx) = crossbeam::channel::unbounded();
    
    let mut runtime = runtime::NodeRuntime {
        state: runtime::NodeState::Booting,
        tick_counter: 0,
        shard_engine,
        shard_tx,
        shard_rx,
        result_tx,
        result_rx,
        shutdown_flag,
    };
    
    runtime.run().unwrap_or_else(|e| {
        tracing::error!("Runtime error: {}", e);
        std::process::exit(1);
    });
    
    runtime.shutdown().expect("Teardown failed");
    
    tracing::info!("Node daemon terminated gracefully.");
    std::process::exit(0);
}
