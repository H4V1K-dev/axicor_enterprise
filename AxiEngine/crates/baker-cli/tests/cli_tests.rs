//! Integration tests for axi-baker CLI.

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

const INVALID_SHARD_TOML: &str = r#"
[dimensions]
w = "string-is-invalid"
"#;

fn get_temp_file_path(suffix: &str) -> PathBuf {
    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("baker_cli_test_{}_{}", rand, suffix));
    temp
}

fn write_temp_file(content: &str, suffix: &str) -> PathBuf {
    let path = get_temp_file_path(suffix);
    let mut f = File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

fn get_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_axi-baker"))
}

#[test]
fn test_cli_help_renders() {
    let output = Command::new(get_bin_path()).arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(stdout_str.contains("axi-baker"));
    assert!(stdout_str.contains("bake-local"));
}

#[test]
fn test_cli_missing_required_args() {
    // Missing seed, voxel-size, etc.
    let output = Command::new(get_bin_path())
        .args(["bake-local", "--shard", "dummy.toml", "--out", "dummy.axic"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_cli_invalid_toml() {
    let invalid_toml_path = write_temp_file(INVALID_SHARD_TOML, "invalid.toml");
    let out_path = get_temp_file_path("out.axic");

    let output = Command::new(get_bin_path())
        .args([
            "bake-local",
            "--shard",
            invalid_toml_path.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--seed",
            "42",
            "--voxel-size-um",
            "1.0",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));

    let _ = remove_file(invalid_toml_path);
}

#[test]
fn test_cli_valid_toml_creates_axic() {
    let valid_toml_path = write_temp_file(VALID_SHARD_TOML, "valid.toml");
    let out_path = get_temp_file_path("out.axic");

    let output = Command::new(get_bin_path())
        .args([
            "bake-local",
            "--shard",
            valid_toml_path.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--seed",
            "42",
            "--voxel-size-um",
            "1.0",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "CLI run failed: {:?}", output);
    assert!(out_path.exists());

    // 8. Created .axic is readable by vfs
    let archive = vfs::AxicArchive::open(&out_path);
    assert!(
        archive.is_ok(),
        "Generated .axic is corrupt: {:?}",
        archive.err()
    );
    let archive = archive.unwrap();
    assert!(archive.get_file("state.bin").is_some());
    assert!(archive.get_file("axons.bin").is_some());
    assert!(archive.get_file("paths.bin").is_some());
    assert!(archive.get_file("variant_table.bin").is_some());

    // 9. Source TOML is not mutated
    let original_content = std::fs::read_to_string(&valid_toml_path).unwrap();
    assert_eq!(original_content, VALID_SHARD_TOML);

    let _ = remove_file(valid_toml_path);
    let _ = remove_file(out_path);
}

#[test]
fn test_cli_overwrite_without_force_fails() {
    let valid_toml_path = write_temp_file(VALID_SHARD_TOML, "valid.toml");
    let out_path = write_temp_file("pre-existing content", "out.axic");

    let output = Command::new(get_bin_path())
        .args([
            "bake-local",
            "--shard",
            valid_toml_path.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--seed",
            "42",
            "--voxel-size-um",
            "1.0",
        ])
        .output()
        .unwrap();

    // 5. Existing output without --force fails and does not alter the file
    assert_eq!(output.status.code(), Some(3));
    let current_content = std::fs::read(&out_path).unwrap();
    assert_eq!(current_content, b"pre-existing content");

    let _ = remove_file(valid_toml_path);
    let _ = remove_file(out_path);
}

#[test]
fn test_cli_overwrite_with_force_succeeds() {
    let valid_toml_path = write_temp_file(VALID_SHARD_TOML, "valid.toml");
    let out_path = write_temp_file("pre-existing content", "out.axic");

    let output = Command::new(get_bin_path())
        .args([
            "bake-local",
            "--shard",
            valid_toml_path.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--seed",
            "42",
            "--voxel-size-um",
            "1.0",
            "--force",
        ])
        .output()
        .unwrap();

    // 6. Existing output with --force succeeds and overwrites the file
    assert!(output.status.success());
    let current_content = std::fs::read(&out_path).unwrap();
    assert_ne!(current_content, b"pre-existing content");

    let _ = remove_file(valid_toml_path);
    let _ = remove_file(out_path);
}

#[test]
fn test_cli_json_mode_outputs_valid_json() {
    let valid_toml_path = write_temp_file(VALID_SHARD_TOML, "valid.toml");
    let out_path = get_temp_file_path("out.axic");

    let output = Command::new(get_bin_path())
        .args([
            "bake-local",
            "--shard",
            valid_toml_path.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--seed",
            "42",
            "--voxel-size-um",
            "1.0",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout_str).unwrap();

    assert!(parsed.get("total_somas").is_some());
    assert!(parsed.get("total_axons").is_some());
    assert!(parsed.get("total_synapses").is_some());
    assert!(parsed.get("dropped_candidates").is_some());

    let _ = remove_file(valid_toml_path);
    let _ = remove_file(out_path);
}

#[test]
fn test_cli_relative_out_filename() {
    let valid_toml_path = write_temp_file(VALID_SHARD_TOML, "valid.toml");
    let out_filename = "relative_out_test.axic";
    let out_path = PathBuf::from(out_filename);
    if out_path.exists() {
        let _ = remove_file(&out_path);
    }

    let output = Command::new(get_bin_path())
        .args([
            "bake-local",
            "--shard",
            valid_toml_path.to_str().unwrap(),
            "--out",
            out_filename,
            "--seed",
            "42",
            "--voxel-size-um",
            "1.0",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI relative out run failed: {:?}",
        output
    );
    assert!(out_path.exists());

    // Verify VFS readability
    let archive = vfs::AxicArchive::open(&out_path);
    assert!(archive.is_ok());

    let _ = remove_file(valid_toml_path);
    let _ = remove_file(out_path);
}

#[test]
fn test_cli_production_dependency_guard() {
    let mut cargo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cargo_path.push("Cargo.toml");
    let cargo_content = std::fs::read_to_string(cargo_path).unwrap();

    let prod_deps_start = cargo_content.find("[dependencies]").unwrap();
    let dev_deps_start = cargo_content.find("[dev-dependencies]").unwrap();
    let prod_deps_section = &cargo_content[prod_deps_start..dev_deps_start];

    // 10. Verify we do not import boot, runtime, compute, node, ipc, net in production dependencies
    assert!(!prod_deps_section.contains("boot"));
    assert!(!prod_deps_section.contains("runtime"));
    assert!(!prod_deps_section.contains("compute"));
    assert!(!prod_deps_section.contains("node"));
    assert!(!prod_deps_section.contains("ipc"));
    assert!(!prod_deps_section.contains("net"));

    // Guard verifying boot is not present in dev-dependencies or anywhere else
    assert!(!cargo_content.contains("boot"));
}
