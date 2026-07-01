//! Node execution engine entrypoint.
//!
//! Orchestrates bootstrapping and execution loops for local neuromorphic shard simulations.

use std::fmt;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use clap::{Parser, Subcommand};
use serde::Serialize;

use boot::{bootstrap_local_shard_engine, BackendPreference, LocalShardComputeInput};
use runtime::{LocalRuntime, LocalRuntimeConfig};

#[derive(Debug)]
enum CliError {
    Validation(String),
    Boot(String),
    RuntimeInit(String),
    RuntimeExecution(String),
    Io(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Validation(msg) => write!(f, "Validation error: {}", msg),
            CliError::Boot(msg) => write!(f, "Boot failed: {}", msg),
            CliError::RuntimeInit(msg) => write!(f, "Runtime initialization failed: {}", msg),
            CliError::RuntimeExecution(msg) => write!(f, "Runtime execution failed: {}", msg),
            CliError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for CliError {}

#[derive(Parser)]
#[command(name = "axi-node", about = "AxiEngine simulation runner CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs a local single-shard simulation from .axic archive
    #[command(name = "run-local")]
    RunLocal {
        /// Path to the validated .axic archive
        #[arg(long)]
        archive: PathBuf,

        /// Total simulation ticks to execute
        #[arg(long)]
        ticks: u64,

        /// Tick size within one batch step
        #[arg(long = "batch-ticks")]
        batch_ticks: u32,

        /// Maximum generated spikes count allocated per tick
        #[arg(long = "max-spikes-per-tick")]
        max_spikes_per_tick: u32,

        /// Target compute platform backend
        #[arg(long, default_value = "cpu")]
        backend: String,

        /// Output metrics report in JSON format to stdout
        #[arg(long)]
        json: bool,

        /// Path to export batch/tick CSV logs
        #[arg(long = "csv-dir")]
        csv_dir: Option<PathBuf>,
    },
}

#[derive(Serialize)]
struct NodeSummaryReport {
    backend_kind: String,
    total_ticks: u64,
    total_batches: u64,
    total_generated_spikes: u64,
    total_output_spikes_written: u64,
    total_dropped_spikes: u64,
    max_generated_per_batch: u32,
    max_output_per_tick: u32,
    nonzero_output_ticks: u64,
    mean_backend_execution_time_us: f64,
    wall_time_us: u64,
    final_runtime_state: String,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::RunLocal {
            archive,
            ticks,
            batch_ticks,
            max_spikes_per_tick,
            backend,
            json,
            csv_dir,
        } => {
            // 1. Argument validation
            if ticks == 0 {
                return Err(Box::new(CliError::Validation(
                    "ticks must be greater than 0".to_string(),
                )));
            }
            if batch_ticks == 0 {
                return Err(Box::new(CliError::Validation(
                    "batch-ticks must be greater than 0".to_string(),
                )));
            }
            if max_spikes_per_tick == 0 {
                return Err(Box::new(CliError::Validation(
                    "max-spikes-per-tick must be greater than 0".to_string(),
                )));
            }

            let pref = match backend.to_lowercase().as_str() {
                "cpu" => BackendPreference::Cpu,
                "cuda" => BackendPreference::Cuda { device_id: 0 },
                "auto" => BackendPreference::Auto,
                _ => {
                    return Err(Box::new(CliError::Validation(format!(
                        "invalid backend preference '{}'. Supported: cpu, cuda, auto",
                        backend
                    ))))
                }
            };

            // 2. Bootstrapping ShardEngine
            let boot_input = LocalShardComputeInput {
                archive_path: archive,
                backend_preference: pref.clone(),
                virtual_offset: 0,
                total_ghosts: 0,
            };

            let (engine, bundle) = bootstrap_local_shard_engine(&boot_input)
                .map_err(|e| CliError::Boot(e.to_string()))?;

            // Determine active backend from ShardEngine BEFORE transferring ownership
            let active_backend_kind = engine.backend_kind();
            let active_backend_str = format!("{:?}", active_backend_kind);

            // 3. Creating LocalRuntime
            let mapped_somas = (0..bundle.spec.total_axons).collect();
            let rt_config = LocalRuntimeConfig {
                sync_batch_ticks: batch_ticks,
                v_seg: 1,
                dopamine: 0,
                max_spikes_per_tick,
                virtual_offset: 0,
                num_virtual_axons: 0,
                input_words_per_tick: 0,
                mapped_soma_ids: mapped_somas,
            };

            let mut runtime = LocalRuntime::new(engine, rt_config)
                .map_err(|e| CliError::RuntimeInit(e.to_string()))?;

            // 4. Set up CSV writers if requested
            let mut csv_batches_file = None;
            let mut csv_outputs_file = None;
            let mut csv_spikes_file = None;

            if let Some(ref path) = csv_dir {
                create_dir_all(path).map_err(|e| {
                    CliError::Io(format!(
                        "Failed to create CSV output folder '{}': {}",
                        path.display(),
                        e
                    ))
                })?;

                let batches_path = path.join("node_batches.csv");
                let mut b_file = File::create(&batches_path).map_err(|e| {
                    CliError::Io(format!(
                        "Failed to create file '{}': {}",
                        batches_path.display(),
                        e
                    ))
                })?;
                writeln!(
                    b_file,
                    "batch_idx,tick_count,generated_spikes_count,output_spikes_written,dropped_spikes_count,backend_execution_time_us"
                )?;
                csv_batches_file = Some(b_file);

                let outputs_path = path.join("node_outputs.csv");
                let mut o_file = File::create(&outputs_path).map_err(|e| {
                    CliError::Io(format!(
                        "Failed to create file '{}': {}",
                        outputs_path.display(),
                        e
                    ))
                })?;
                writeln!(o_file, "batch_idx,tick_index,output_spike_count")?;
                csv_outputs_file = Some(o_file);

                let spikes_path = path.join("node_output_spikes.csv");
                let mut s_file = File::create(&spikes_path).map_err(|e| {
                    CliError::Io(format!(
                        "Failed to create file '{}': {}",
                        spikes_path.display(),
                        e
                    ))
                })?;
                writeln!(s_file, "batch_idx,tick_index,slot,soma_id")?;
                csv_spikes_file = Some(s_file);
            }

            // 5. Execution Day-Loop Split By batch_ticks
            let wall_start = Instant::now();

            let full_batches = ticks / (batch_ticks as u64);
            let rem_ticks = ticks % (batch_ticks as u64);
            let total_batches = full_batches + if rem_ticks > 0 { 1 } else { 0 };

            let mut sum_backend_time_us = 0;
            let mut sum_generated_spikes = 0;
            let mut sum_output_spikes = 0;
            let mut sum_dropped_spikes = 0;
            let mut max_gen_per_batch = 0;
            let mut max_out_per_tick = 0;
            let mut nonzero_out_ticks = 0;

            struct BatchMetrics {
                execution_time_us: u64,
                generated_spikes: u32,
                output_spikes: u32,
                dropped_spikes: u64,
                max_output_per_tick: u32,
                nonzero_output_ticks: u64,
            }

            let mut batch_idx = 0;
            let mut current_tick_offset = 0;

            let run_single_batch =
                |runtime: &mut LocalRuntime,
                 size: u32,
                 idx: u64,
                 tick_offset: &mut u64,
                 csv_b: &mut Option<File>,
                 csv_o: &mut Option<File>,
                 csv_s: &mut Option<File>|
                 -> Result<BatchMetrics, Box<dyn std::error::Error>> {
                    let report = runtime
                        .run_empty_batch_with_ticks(size)
                        .map_err(|e| CliError::RuntimeExecution(e.to_string()))?;

                    let res = report.batch_result;
                    let mut nz_ticks = 0;
                    let mut local_max_out = 0;

                    let max_spikes = max_spikes_per_tick as usize;

                    for (tick_local_idx, &cnt) in report.output_spike_counts.iter().enumerate() {
                        let global_tick = *tick_offset + (tick_local_idx as u64);
                        if cnt > 0 {
                            nz_ticks += 1;
                            if cnt > local_max_out {
                                local_max_out = cnt;
                            }
                        }
                        if let Some(ref mut f) = csv_o {
                            writeln!(f, "{},{},{}", idx, global_tick, cnt)?;
                        }
                        if let Some(ref mut f) = csv_s {
                            for slot in 0..cnt as usize {
                                let idx_spikes = tick_local_idx * max_spikes + slot;
                                let soma_id = report.output_spikes[idx_spikes];
                                writeln!(f, "{},{},{},{}", idx, global_tick, slot, soma_id)?;
                            }
                        }
                    }

                    if let Some(ref mut f) = csv_b {
                        writeln!(
                            f,
                            "{},{},{},{},{},{}",
                            idx,
                            size,
                            res.generated_spikes_count,
                            res.output_spikes_written,
                            res.dropped_spikes_count,
                            res.execution_time_us
                        )?;
                    }

                    *tick_offset += size as u64;

                    Ok(BatchMetrics {
                        execution_time_us: res.execution_time_us,
                        generated_spikes: res.generated_spikes_count,
                        output_spikes: res.output_spikes_written,
                        dropped_spikes: res.dropped_spikes_count as u64,
                        max_output_per_tick: local_max_out,
                        nonzero_output_ticks: nz_ticks,
                    })
                };

            for _ in 0..full_batches {
                let metrics = run_single_batch(
                    &mut runtime,
                    batch_ticks,
                    batch_idx,
                    &mut current_tick_offset,
                    &mut csv_batches_file,
                    &mut csv_outputs_file,
                    &mut csv_spikes_file,
                )?;
                sum_backend_time_us += metrics.execution_time_us;
                sum_generated_spikes += metrics.generated_spikes as u64;
                sum_output_spikes += metrics.output_spikes as u64;
                sum_dropped_spikes += metrics.dropped_spikes;
                if metrics.generated_spikes > max_gen_per_batch {
                    max_gen_per_batch = metrics.generated_spikes;
                }
                if metrics.max_output_per_tick > max_out_per_tick {
                    max_out_per_tick = metrics.max_output_per_tick;
                }
                nonzero_out_ticks += metrics.nonzero_output_ticks;
                batch_idx += 1;
            }

            if rem_ticks > 0 {
                let metrics = run_single_batch(
                    &mut runtime,
                    rem_ticks as u32,
                    batch_idx,
                    &mut current_tick_offset,
                    &mut csv_batches_file,
                    &mut csv_outputs_file,
                    &mut csv_spikes_file,
                )?;
                sum_backend_time_us += metrics.execution_time_us;
                sum_generated_spikes += metrics.generated_spikes as u64;
                sum_output_spikes += metrics.output_spikes as u64;
                sum_dropped_spikes += metrics.dropped_spikes;
                if metrics.generated_spikes > max_gen_per_batch {
                    max_gen_per_batch = metrics.generated_spikes;
                }
                if metrics.max_output_per_tick > max_out_per_tick {
                    max_out_per_tick = metrics.max_output_per_tick;
                }
                nonzero_out_ticks += metrics.nonzero_output_ticks;
            }

            let wall_elapsed = wall_start.elapsed().as_micros() as u64;

            let final_state = format!("{:?}", runtime.state());

            // Determine active backend from actual engine backend_kind
            let active_backend_kind = active_backend_str.to_uppercase();

            let _ = runtime.shutdown();

            let mean_backend_execution_time_us = if total_batches > 0 {
                sum_backend_time_us as f64 / total_batches as f64
            } else {
                0.0
            };

            let summary = NodeSummaryReport {
                backend_kind: active_backend_kind.to_string(),
                total_ticks: ticks,
                total_batches,
                total_generated_spikes: sum_generated_spikes,
                total_output_spikes_written: sum_output_spikes,
                total_dropped_spikes: sum_dropped_spikes,
                max_generated_per_batch: max_gen_per_batch,
                max_output_per_tick: max_out_per_tick,
                nonzero_output_ticks: nonzero_out_ticks,
                mean_backend_execution_time_us,
                wall_time_us: wall_elapsed,
                final_runtime_state: final_state,
            };

            // 6. CSV summary write
            if let Some(ref path) = csv_dir {
                let summary_path = path.join("node_summary.csv");
                let mut s_file = File::create(&summary_path).map_err(|e| {
                    CliError::Io(format!(
                        "Failed to create file '{}': {}",
                        summary_path.display(),
                        e
                    ))
                })?;
                writeln!(s_file, "key,value")?;
                writeln!(s_file, "backend_kind,{}", summary.backend_kind)?;
                writeln!(s_file, "total_ticks,{}", summary.total_ticks)?;
                writeln!(s_file, "total_batches,{}", summary.total_batches)?;
                writeln!(
                    s_file,
                    "total_generated_spikes,{}",
                    summary.total_generated_spikes
                )?;
                writeln!(
                    s_file,
                    "total_output_spikes_written,{}",
                    summary.total_output_spikes_written
                )?;
                writeln!(
                    s_file,
                    "total_dropped_spikes,{}",
                    summary.total_dropped_spikes
                )?;
                writeln!(
                    s_file,
                    "max_generated_per_batch,{}",
                    summary.max_generated_per_batch
                )?;
                writeln!(
                    s_file,
                    "max_output_per_tick,{}",
                    summary.max_output_per_tick
                )?;
                writeln!(
                    s_file,
                    "nonzero_output_ticks,{}",
                    summary.nonzero_output_ticks
                )?;
                writeln!(
                    s_file,
                    "mean_backend_execution_time_us,{}",
                    summary.mean_backend_execution_time_us
                )?;
                writeln!(s_file, "wall_time_us,{}", summary.wall_time_us)?;
                writeln!(
                    s_file,
                    "final_runtime_state,{}",
                    summary.final_runtime_state
                )?;
            }

            // 7. Output Result Summary
            if json {
                let json_str = serde_json::to_string(&summary)?;
                println!("{}", json_str);
            } else {
                println!("\n========================================================");
                println!("  AXIENGINE NODE SIMULATION RUNNER COMPLETE");
                println!("========================================================");
                println!("  Backend Preference              : {}", backend);
                println!(
                    "  Active Backend Kind             : {}",
                    summary.backend_kind
                );
                println!(
                    "  Total Ticks Run                 : {}",
                    summary.total_ticks
                );
                println!(
                    "  Total Batches executed          : {}",
                    summary.total_batches
                );
                println!(
                    "  Total Generated Spikes          : {}",
                    summary.total_generated_spikes
                );
                println!(
                    "  Total Output Spikes Written     : {}",
                    summary.total_output_spikes_written
                );
                println!(
                    "  Total Dropped Spikes            : {}",
                    summary.total_dropped_spikes
                );
                println!(
                    "  Max Generated per Batch         : {}",
                    summary.max_generated_per_batch
                );
                println!(
                    "  Max Output per Tick             : {}",
                    summary.max_output_per_tick
                );
                println!(
                    "  Nonzero Output Ticks            : {}",
                    summary.nonzero_output_ticks
                );
                println!(
                    "  Mean Backend Exec Time (us)     : {:.2}",
                    summary.mean_backend_execution_time_us
                );
                println!(
                    "  Total Wall Clock Time (us)      : {}",
                    summary.wall_time_us
                );
                println!(
                    "  Final Runtime State             : {}",
                    summary.final_runtime_state
                );
                println!("========================================================\n");
            }
        }
    }

    Ok(())
}

fn main() {
    // Custom panic hook to return Exit Code 70 for panics
    std::panic::set_hook(Box::new(|info| {
        eprintln!("Internal Node Engine Panic: {}", info);
        std::process::exit(70);
    }));

    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        let code = if e.is::<CliError>() {
            match e.downcast_ref::<CliError>().unwrap() {
                CliError::Validation(_) => 2,
                CliError::Boot(_) => 10,
                CliError::RuntimeInit(_) => 20,
                CliError::RuntimeExecution(_) => 21,
                CliError::Io(_) => 3,
            }
        } else {
            70 // Internal default error mapping
        };
        std::process::exit(code);
    }
}
