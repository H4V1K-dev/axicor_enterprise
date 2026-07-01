//! Connectome compiler CLI.
//!
//! Exposes a command-line interface for invoking the AOT connectome baker.

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde::Serialize;

use baker::{bake_local_shard_axic, LocalShardBakeInput};
use types::MasterSeed;

#[derive(Debug)]
enum CliError {
    ValidationError(String),
    IoError(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            CliError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for CliError {}

#[derive(Parser)]
#[command(name = "axi-baker", about = "AxiEngine connectome compiler CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Bakes a single shard into .axic
    #[command(name = "bake-local")]
    BakeLocal {
        /// Path to the shard TOML configuration file
        #[arg(long)]
        shard: PathBuf,

        /// Output path for the baked .axic archive
        #[arg(long)]
        out: PathBuf,

        /// Seed for topology random generators
        #[arg(long)]
        seed: u64,

        /// Size of voxels in micrometers
        #[arg(long = "voxel-size-um")]
        voxel_size_um: f32,

        /// Overwrite output file if it exists
        #[arg(long)]
        force: bool,

        /// Output results in JSON format to stdout
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize)]
struct CliBakeReport {
    total_somas: u32,
    total_axons: u32,
    total_synapses: u32,
    dropped_candidates: u64,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BakeLocal {
            shard,
            out,
            seed,
            voxel_size_um,
            force,
            json,
        } => {
            // 1. Check if output file already exists
            if out.exists() && !force {
                return Err(Box::new(CliError::IoError(format!(
                    "Output file already exists at '{}'. Use --force to overwrite.",
                    out.display()
                ))));
            }

            // 2. Read shard TOML configuration
            eprintln!("Reading shard configuration from '{}'...", shard.display());
            let shard_config = config::load_shard_from_file(&shard)
                .map_err(|e| CliError::ValidationError(e.to_string()))?;

            // 3. Validate configuration
            eprintln!("Validating shard configuration...");
            config::validate_shard(&shard_config)
                .map_err(|e| CliError::ValidationError(e.to_string()))?;

            // 4. Perform AOT baking
            eprintln!("Baking local shard topology...");
            let bake_input = LocalShardBakeInput {
                shard_config: &shard_config,
                master_seed: MasterSeed(seed),
                voxel_size_um,
            };

            let (axic_bytes, report) = bake_local_shard_axic(&bake_input)
                .map_err(|e| CliError::ValidationError(e.to_string()))?;

            // 5. Atomic write using NamedTempFile in the target directory
            let parent_dir = match out.parent() {
                Some(p) if !p.as_os_str().is_empty() => p,
                _ => Path::new("."),
            };
            let mut temp_file = tempfile::Builder::new()
                .prefix("axi_baker_tmp")
                .suffix(".axic")
                .tempfile_in(parent_dir)
                .map_err(|e| CliError::IoError(format!("Failed to create temp file: {}", e)))?;

            eprintln!("Writing .axic archive bytes...");
            temp_file
                .write_all(&axic_bytes)
                .map_err(|e| CliError::IoError(format!("Failed to write to temp file: {}", e)))?;

            if force {
                temp_file.persist(&out).map_err(|e| {
                    CliError::IoError(format!("Failed to persist output file: {}", e))
                })?;
            } else {
                temp_file.persist_noclobber(&out).map_err(|e| {
                    CliError::IoError(format!(
                        "Output file already exists or failed to persist: {}",
                        e
                    ))
                })?;
            }

            eprintln!("Bake successfully completed.");

            // 6. JSON Reporting
            if json {
                let cli_report = CliBakeReport {
                    total_somas: report.total_somas,
                    total_axons: report.total_axons,
                    total_synapses: report.total_synapses,
                    dropped_candidates: report.dropped_candidates,
                };
                let json_out = serde_json::to_string(&cli_report)?;
                println!("{}", json_out);
            }
        }
    }

    Ok(())
}

fn main() {
    // Custom panic hook to return Exit Code 4 for panics
    std::panic::set_hook(Box::new(|info| {
        eprintln!("Internal Engine Panic: {}", info);
        std::process::exit(4);
    }));

    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        let code = if e.is::<CliError>() {
            match e.downcast_ref::<CliError>().unwrap() {
                CliError::ValidationError(_) => 1,
                CliError::IoError(_) => 3,
            }
        } else {
            1 // Default mapping for other errors (e.g. config parse errors or baker pipeline errors)
        };
        std::process::exit(code);
    }
}
