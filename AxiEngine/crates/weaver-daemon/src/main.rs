//! CLI entry point for executing Night Phase synaptogenesis tasks.
//!
//! Exposes a clap-parsed interface that attaches to shared memory and executes once.

use clap::Parser;
use weaver_daemon::{run_night_pipeline, NightBufferSource, WeaverJobRequest};

#[derive(Parser, Debug)]
#[command(author, version, about = "Weaver Daemon CLI for AxiEngine")]
struct Args {
    /// FNV-1a hash of the target zone to coordinate.
    #[arg(short, long)]
    zone_hash: u32,

    /// Unique shard identification index.
    #[arg(short, long, default_value_t = 0)]
    shard_id: u32,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let shm_name = ipc::shm_segment_name(args.zone_hash);
    let mut source = NightBufferSource::ShmAttachment { shm_name };

    let req = WeaverJobRequest {
        shard_id: args.shard_id,
        zone_hash: args.zone_hash,
        night_epoch: 0,
        master_seed: [0; 32],
        prune_threshold: 0,
        max_sprouts: 0,
        w_distance: 0,
        w_power: 0,
        w_explore: 0,
        initial_synapse_weight: 0,
        has_growth_context: false,
    };

    println!(
        "Attaching to shared memory segment for zone 0x{:08X}...",
        args.zone_hash
    );

    match run_night_pipeline(&req, None, &mut source) {
        Ok((report, handovers)) => {
            println!("Weaver pipeline completed successfully.");
            println!("Report: {:?}", report);
            println!("Handovers generated: {}", handovers.len());
        }
        Err(e) => {
            eprintln!("Pipeline execution failed: {:?}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
