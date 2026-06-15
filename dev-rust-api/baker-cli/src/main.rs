//! baker-cli — console frontend/orchestrator.
//!
//! # CLI Invariants
//! - **INV-BCLI-001**: Zero GPU Runtime link.
//!   This offline compiler must remain strictly GPU-blind. It must never link to compute-api,
//!   compute-cuda, compute-hip, or any GPU driver.
//! - **INV-BCLI-002**: Zero Panic Orchestration.
//!   All user-facing errors must be printed to stderr and cause process termination with exit code 1
//!   instead of panicking and dumping a stack trace.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "baker-cli")]
#[command(about = "Axicor Graph Baker & Edge Model Compiler CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Compile (AOT) TOML configurations into a binary .axic archive
    Bake {
        /// Path to the directory containing configuration TOMLs
        config_dir: PathBuf,
        /// Path where the output .axic archive should be saved
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Distill the compiled archive's synapses and export C headers for MCU target
    Distill {
        /// Path to the compiled .axic archive
        archive: PathBuf,
        /// Destination directory for SRAM/Flash binaries and .h headers
        #[arg(short, long)]
        out_dir: PathBuf,
        /// Target dendrite slot budget K
        #[arg(short = 'k', long, default_value_t = 32)]
        target_slots: usize,
    },
}

fn run() -> anyhow::Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Bake { config_dir, output } => {
            baker::bake(&config_dir, &output)?;
        }
        Commands::Distill {
            archive,
            out_dir,
            target_slots,
        } => {
            let ax_archive = vfs::AxicArchive::open(&archive)?;
            let config = edge_model::EdgeConfig {
                target_dendrite_slots: target_slots,
            };
            let model = edge_model::distill::convert_archive(&ax_archive, &config)?;
            edge_model::export::export_c_headers(&model, &out_dir)?;
        }
    }
    Ok(())
}

fn main() {
    match run() {
        Ok(()) => {
            println!("Operation completed successfully.");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Fatal Error: {:?}", e);
            std::process::exit(1);
        }
    }
}
