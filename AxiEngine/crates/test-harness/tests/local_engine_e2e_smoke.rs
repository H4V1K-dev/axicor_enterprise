//! End-to-end integration smoke test for AxiEngine local pipeline.
//!
//! Generates a configuration, bakes it using `axi-baker`,
//! runs simulation using `axi-node`, and checks telemetry and CSV output.

#![cfg(feature = "local-engine-e2e-smoke")]

use std::fs::{create_dir_all, remove_file, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const SMOKE_SHARD_TOML: &str = r#"
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
fatigue_capacity = 255
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

fn get_workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates
    path.pop(); // AxiEngine
    path
}

fn get_baker_bin_path() -> PathBuf {
    let mut path = get_workspace_root();
    path.push("target");
    path.push("debug");
    #[cfg(windows)]
    path.push("axi-baker.exe");
    #[cfg(not(windows))]
    path.push("axi-baker");
    path
}

fn get_node_bin_path() -> PathBuf {
    let mut path = get_workspace_root();
    path.push("target");
    path.push("debug");
    #[cfg(windows)]
    path.push("axi-node.exe");
    #[cfg(not(windows))]
    path.push("axi-node");
    path
}

fn get_temp_file_path(suffix: &str) -> PathBuf {
    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("e2e_smoke_{}_{}", rand, suffix));
    temp
}

fn write_temp_file(content: &str, suffix: &str) -> PathBuf {
    let path = get_temp_file_path(suffix);
    let mut f = File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
#[ignore]
fn test_local_engine_e2e_smoke() {
    let workspace = get_workspace_root();
    let artifacts_dir = workspace.join("artifacts").join("local_engine_e2e");
    create_dir_all(&artifacts_dir).unwrap();

    let baker_bin = get_baker_bin_path();
    let node_bin = get_node_bin_path();

    assert!(
        baker_bin.exists(),
        "axi-baker binary not found at {}",
        baker_bin.display()
    );
    assert!(
        node_bin.exists(),
        "axi-node binary not found at {}",
        node_bin.display()
    );

    // 1. Write temporary TOML configuration file
    let toml_path = write_temp_file(SMOKE_SHARD_TOML, "shard.toml");
    let axic_path = get_temp_file_path("shard.axic");

    // 2. Execute axi-baker
    let baker_output = Command::new(&baker_bin)
        .args([
            "bake-local",
            "--shard",
            toml_path.to_str().unwrap(),
            "--out",
            axic_path.to_str().unwrap(),
            "--seed",
            "42",
            "--voxel-size-um",
            "1.0",
            "--json",
        ])
        .output()
        .expect("Failed to execute axi-baker");

    assert!(
        baker_output.status.success(),
        "axi-baker failed: {:?}",
        baker_output
    );
    assert!(axic_path.exists(), "Output .axic file was not created");

    // 3. Execute axi-node
    let node_output = Command::new(&node_bin)
        .args([
            "run-local",
            "--archive",
            axic_path.to_str().unwrap(),
            "--ticks",
            "35",
            "--batch-ticks",
            "10",
            "--max-spikes-per-tick",
            "100",
            "--backend",
            "cpu",
            "--json",
            "--csv-dir",
            artifacts_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute axi-node");

    assert!(
        node_output.status.success(),
        "axi-node failed: {:?}",
        node_output
    );

    // 4. Parse node output summary JSON
    let node_stdout_str = String::from_utf8_lossy(&node_output.stdout);
    let parsed_summary: serde_json::Value =
        serde_json::from_str(&node_stdout_str).expect("axi-node output is not a valid JSON");

    // Validate metrics
    assert!(parsed_summary.get("backend_kind").is_some());
    assert_eq!(
        parsed_summary.get("total_ticks").unwrap().as_u64(),
        Some(35)
    );
    assert_eq!(
        parsed_summary.get("total_batches").unwrap().as_u64(),
        Some(4)
    );
    assert!(parsed_summary.get("wall_time_us").is_some());
    assert!(parsed_summary.get("final_runtime_state").is_some());
    assert!(parsed_summary.get("total_generated_spikes").is_some());
    assert!(parsed_summary.get("total_output_spikes_written").is_some());

    // 5. Verify CSV files exist and are not empty
    let batches_csv = artifacts_dir.join("node_batches.csv");
    let outputs_csv = artifacts_dir.join("node_outputs.csv");
    let spikes_csv = artifacts_dir.join("node_output_spikes.csv");
    let summary_csv = artifacts_dir.join("node_summary.csv");

    assert!(batches_csv.exists(), "node_batches.csv missing");
    assert!(outputs_csv.exists(), "node_outputs.csv missing");
    assert!(spikes_csv.exists(), "node_output_spikes.csv missing");
    assert!(summary_csv.exists(), "node_summary.csv missing");

    assert!(std::fs::metadata(&batches_csv).unwrap().len() > 0);
    assert!(std::fs::metadata(&outputs_csv).unwrap().len() > 0);
    assert!(std::fs::metadata(&spikes_csv).unwrap().len() > 0);
    assert!(std::fs::metadata(&summary_csv).unwrap().len() > 0);

    // Save local_engine_e2e_summary.json in the artifacts directory
    let summary_json_path = artifacts_dir.join("local_engine_e2e_summary.json");
    let mut sj_file = File::create(&summary_json_path).unwrap();
    sj_file.write_all(node_stdout_str.as_bytes()).unwrap();

    // 6. Clean up temporary files on success
    let _ = remove_file(toml_path);
    let _ = remove_file(axic_path);
}

const ACTIVE_SMOKE_SHARD_TOML: &str = r#"
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
threshold = 10
rest_potential = -70
leak_shift = 1
ahp_amplitude = 5
[neuron_types.timing]
refractory_period = 2
fatigue_capacity = 255
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
initial_synapse_weight = 2000
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
spontaneous_firing_period_ticks = 2

[[layers]]
name = "L1"
height_pct = 1.0
density = 0.2
[[layers.composition]]
type_name = "TypeA"
share = 1.0
"#;

#[test]
#[ignore]
fn test_local_engine_active_e2e_smoke() {
    let workspace = get_workspace_root();
    let artifacts_dir = workspace.join("artifacts").join("local_engine_active_e2e");
    create_dir_all(&artifacts_dir).unwrap();

    let baker_bin = get_baker_bin_path();
    let node_bin = get_node_bin_path();

    assert!(
        baker_bin.exists(),
        "axi-baker binary not found at {}",
        baker_bin.display()
    );
    assert!(
        node_bin.exists(),
        "axi-node binary not found at {}",
        node_bin.display()
    );

    // 1. Write temporary TOML configuration file (stimulated mode)
    let toml_path = write_temp_file(ACTIVE_SMOKE_SHARD_TOML, "active_shard.toml");
    let axic_path = get_temp_file_path("active_shard.axic");

    // 2. Execute axi-baker
    let baker_output = Command::new(&baker_bin)
        .args([
            "bake-local",
            "--shard",
            toml_path.to_str().unwrap(),
            "--out",
            axic_path.to_str().unwrap(),
            "--seed",
            "42",
            "--voxel-size-um",
            "1.0",
            "--json",
        ])
        .output()
        .expect("Failed to execute axi-baker");

    assert!(
        baker_output.status.success(),
        "axi-baker failed: {:?}",
        baker_output
    );

    // 3. Execute axi-node
    let node_output = Command::new(&node_bin)
        .args([
            "run-local",
            "--archive",
            axic_path.to_str().unwrap(),
            "--ticks",
            "35",
            "--batch-ticks",
            "10",
            "--max-spikes-per-tick",
            "100",
            "--backend",
            "cpu",
            "--json",
            "--csv-dir",
            artifacts_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute axi-node");

    assert!(
        node_output.status.success(),
        "axi-node failed: {:?}",
        node_output
    );

    // 4. Parse node output summary JSON
    let node_stdout_str = String::from_utf8_lossy(&node_output.stdout);
    let parsed_summary: serde_json::Value =
        serde_json::from_str(&node_stdout_str).expect("axi-node output is not a valid JSON");

    // Check that we got positive spikes (due to spontaneous firing and low threshold)
    let gen_spikes = parsed_summary
        .get("total_generated_spikes")
        .unwrap()
        .as_u64()
        .unwrap();
    let out_spikes = parsed_summary
        .get("total_output_spikes_written")
        .unwrap()
        .as_u64()
        .unwrap();

    assert!(
        gen_spikes > 0,
        "Expected positive generated spikes, got {}",
        gen_spikes
    );
    assert!(
        out_spikes > 0,
        "Expected positive output spikes written, got {}",
        out_spikes
    );

    // 5. Verify CSV files exist and are not empty
    let batches_csv = artifacts_dir.join("node_batches.csv");
    let outputs_csv = artifacts_dir.join("node_outputs.csv");
    let spikes_csv = artifacts_dir.join("node_output_spikes.csv");
    let summary_csv = artifacts_dir.join("node_summary.csv");

    assert!(batches_csv.exists());
    assert!(outputs_csv.exists());
    assert!(spikes_csv.exists());
    assert!(summary_csv.exists());

    assert!(std::fs::metadata(&batches_csv).unwrap().len() > 0);
    assert!(std::fs::metadata(&outputs_csv).unwrap().len() > 0);
    assert!(std::fs::metadata(&spikes_csv).unwrap().len() > 0);
    assert!(std::fs::metadata(&summary_csv).unwrap().len() > 0);

    // Save local_engine_active_e2e_summary.json in the artifacts directory
    let summary_json_path = artifacts_dir.join("local_engine_active_e2e_summary.json");
    let mut sj_file = File::create(&summary_json_path).unwrap();
    sj_file.write_all(node_stdout_str.as_bytes()).unwrap();

    // 6. Clean up temporary files on success
    let _ = remove_file(toml_path);
    let _ = remove_file(axic_path);
}
