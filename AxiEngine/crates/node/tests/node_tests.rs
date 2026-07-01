//! Integration tests for axi-node simulation runner.

use std::fs::{remove_file, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const VALID_SHARD_TOML: &str = r#"
[dimensions]
w = 20
d = 20
h = 20

[settings]
ghost_capacity = 1024
prune_threshold = 0
max_sprouts = 8
night_interval_ticks = 100
save_checkpoints_interval_ticks = 1000

[[neuron_types]]
name = "TypeA"
[neuron_types.membrane]
threshold = 1000
rest_potential = -70
leak_shift = 1
ahp_amplitude = 5
[neuron_types.timing]
refractory_period = 2
synapse_refractory_period = 2
[neuron_types.signal]
signal_propagation_length = 10
[neuron_types.homeostasis]
homeostasis_penalty = 0
homeostasis_decay = 10
[neuron_types.adaptive_leak]
adaptive_leak_min_shift = 0
adaptive_leak_gain = 0
adaptive_mode = 0
[neuron_types.dopamine]
d1_affinity = 0
d2_affinity = 0
[neuron_types.gsop]
gsop_potentiation = 1
gsop_depression = 1
initial_synapse_weight = 100
is_inhibitory = false
inertia_curve = [1, 1, 1, 1, 1, 1, 1, 1]
[neuron_types.growth]
steering_fov_deg = 45.0
steering_radius_um = 10.0
steering_weight_inertia = 0.5
steering_weight_sensor = 0.5
steering_weight_jitter = 0.1
dendrite_radius_um = 5.0
growth_vertical_bias = 0.0
type_affinity = 1.0
dendrite_whitelist = []
sprouting_weight_distance = 1.0
sprouting_weight_power = 1.0
sprouting_weight_explore = 1.0
sprouting_weight_type = 1.0
[neuron_types.spontaneous]
spontaneous_firing_period_ticks = 0

[[layers]]
name = "L1"
height_pct = 1.0
density = 0.2
[[layers.composition]]
type_name = "TypeA"
share = 1.0
"#;

fn get_temp_file_path(suffix: &str) -> PathBuf {
    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("node_test_{}_{}", rand, suffix));
    temp
}

fn write_temp_file(content: &str, suffix: &str) -> PathBuf {
    let path = get_temp_file_path(suffix);
    let mut f = File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

fn get_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_axi-node"))
}

/// Helper to compile a valid .axic archive on disk using config and baker
fn bake_valid_axic() -> PathBuf {
    let toml_path = write_temp_file(VALID_SHARD_TOML, "valid.toml");
    let out_path = get_temp_file_path("out.axic");

    let shard_config = config::load_shard_from_file(&toml_path).unwrap();
    let bake_input = baker::LocalShardBakeInput {
        shard_config: &shard_config,
        master_seed: types::MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (axic_bytes, _report) = baker::bake_local_shard_axic(&bake_input).unwrap();

    let mut f = File::create(&out_path).unwrap();
    f.write_all(&axic_bytes).unwrap();

    let _ = remove_file(toml_path);
    out_path
}

#[test]
fn test_node_help_renders() {
    let output = Command::new(get_bin_path()).arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(stdout_str.contains("axi-node"));
    assert!(stdout_str.contains("run-local"));
}

#[test]
fn test_node_missing_args() {
    let output = Command::new(get_bin_path())
        .args(["run-local", "--archive", "dummy.axic"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_node_invalid_archive_path() {
    let output = Command::new(get_bin_path())
        .args([
            "run-local",
            "--archive",
            "nonexistent_archive.axic",
            "--ticks",
            "10",
            "--batch-ticks",
            "5",
            "--max-spikes-per-tick",
            "100",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(10));
}

#[test]
fn test_node_corrupted_archive() {
    let corrupted_path = write_temp_file("bad container magic bytes", "corrupted.axic");
    let output = Command::new(get_bin_path())
        .args([
            "run-local",
            "--archive",
            corrupted_path.to_str().unwrap(),
            "--ticks",
            "10",
            "--batch-ticks",
            "5",
            "--max-spikes-per-tick",
            "100",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(10));

    let _ = remove_file(corrupted_path);
}

#[test]
fn test_node_invalid_zero_ticks_rejected() {
    let axic_path = bake_valid_axic();
    let output = Command::new(get_bin_path())
        .args([
            "run-local",
            "--archive",
            axic_path.to_str().unwrap(),
            "--ticks",
            "0",
            "--batch-ticks",
            "5",
            "--max-spikes-per-tick",
            "100",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let _ = remove_file(axic_path);
}

#[test]
fn test_node_valid_run_on_cpu() {
    let axic_path = bake_valid_axic();
    let output = Command::new(get_bin_path())
        .args([
            "run-local",
            "--archive",
            axic_path.to_str().unwrap(),
            "--ticks",
            "20",
            "--batch-ticks",
            "5",
            "--max-spikes-per-tick",
            "100",
            "--backend",
            "cpu",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "run failed: {:?}", output);
    let _ = remove_file(axic_path);
}

#[test]
fn test_node_ticks_remainder_partition() {
    let axic_path = bake_valid_axic();
    let output = Command::new(get_bin_path())
        .args([
            "run-local",
            "--archive",
            axic_path.to_str().unwrap(),
            "--ticks",
            "27",
            "--batch-ticks",
            "10",
            "--max-spikes-per-tick",
            "100",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout_str).unwrap();

    assert_eq!(parsed.get("total_ticks").unwrap().as_u64(), Some(27));
    assert_eq!(parsed.get("total_batches").unwrap().as_u64(), Some(3)); // 10, 10, 7

    let _ = remove_file(axic_path);
}

#[test]
fn test_node_json_mode_and_csv_generation() {
    let axic_path = bake_valid_axic();
    let csv_path = get_temp_file_path("csv_out");

    let output = Command::new(get_bin_path())
        .args([
            "run-local",
            "--archive",
            axic_path.to_str().unwrap(),
            "--ticks",
            "15",
            "--batch-ticks",
            "5",
            "--max-spikes-per-tick",
            "100",
            "--json",
            "--csv-dir",
            csv_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "CLI run failed: {:?}", output);
    let stdout_str = String::from_utf8_lossy(&output.stdout);

    // Verify JSON Report
    let parsed: serde_json::Value = serde_json::from_str(&stdout_str).unwrap();
    assert!(parsed.get("backend_kind").is_some());
    assert_eq!(parsed.get("total_ticks").unwrap().as_u64(), Some(15));
    assert_eq!(parsed.get("total_batches").unwrap().as_u64(), Some(3));
    assert!(parsed.get("total_generated_spikes").is_some());
    assert!(parsed.get("wall_time_us").is_some());

    // Verify CSV Reports
    let node_batches_csv = csv_path.join("node_batches.csv");
    let node_outputs_csv = csv_path.join("node_outputs.csv");
    let node_summary_csv = csv_path.join("node_summary.csv");
    let node_output_spikes_csv = csv_path.join("node_output_spikes.csv");

    assert!(node_batches_csv.exists());
    assert!(node_outputs_csv.exists());
    assert!(node_summary_csv.exists());
    assert!(node_output_spikes_csv.exists());

    let b_content = std::fs::read_to_string(&node_batches_csv).unwrap();
    assert!(!b_content.is_empty());
    assert!(b_content.contains("batch_idx,tick_count"));

    let o_content = std::fs::read_to_string(&node_outputs_csv).unwrap();
    assert!(!o_content.is_empty());
    assert!(o_content.contains("batch_idx,tick_index"));

    let s_content = std::fs::read_to_string(&node_summary_csv).unwrap();
    assert!(!s_content.is_empty());
    assert!(s_content.contains("key,value"));
    assert!(s_content.contains("total_ticks,15"));

    let sp_content = std::fs::read_to_string(&node_output_spikes_csv).unwrap();
    assert!(!sp_content.is_empty());
    assert!(sp_content.contains("batch_idx,tick_index,slot,soma_id"));

    let _ = std::fs::remove_dir_all(&csv_path);
    let _ = remove_file(axic_path);
}

#[test]
fn test_node_production_dependency_guard() {
    let mut cargo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cargo_path.push("Cargo.toml");
    let cargo_content = std::fs::read_to_string(cargo_path).unwrap();

    let prod_deps_start = cargo_content.find("[dependencies]").unwrap();
    let dev_deps_start = cargo_content.find("[dev-dependencies]").unwrap();
    let prod_deps_section = &cargo_content[prod_deps_start..dev_deps_start];

    // Check strict L6 limits (no direct ipc, net, wire, protocol, transport, checkpoint, etc.)
    assert!(!prod_deps_section.contains("ipc"));
    assert!(!prod_deps_section.contains("net"));
    assert!(!prod_deps_section.contains("wire"));
    assert!(!prod_deps_section.contains("protocol"));
    assert!(!prod_deps_section.contains("transport"));
    assert!(!prod_deps_section.contains("checkpoint"));
    assert!(!prod_deps_section.contains("system-service"));
}

#[test]
fn test_node_backend_auto_resolves_actual() {
    let axic_path = bake_valid_axic();
    let output = Command::new(get_bin_path())
        .args([
            "run-local",
            "--archive",
            axic_path.to_str().unwrap(),
            "--ticks",
            "10",
            "--batch-ticks",
            "5",
            "--max-spikes-per-tick",
            "100",
            "--backend",
            "auto",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout_str).unwrap();

    // Check that backend_kind is "CPU", representing the resolved cpu backend,
    // rather than "AUTO" or empty.
    assert_eq!(parsed.get("backend_kind").unwrap().as_str(), Some("CPU"));

    let _ = remove_file(axic_path);
}
