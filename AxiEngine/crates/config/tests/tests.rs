use config::*;
use std::fs::File;
use std::io::Write;

const VALID_MODEL_TOML: &str = r#"
[world]
width_um = 1000.0
depth_um = 1000.0
height_um = 100.0

[simulation]
tick_duration_us = 100
total_ticks = 10000
master_seed = "some_seed"
voxel_size_um = 2.0
segment_length_voxels = 10
signal_speed_m_s = 2.0
sync_batch_ticks = 100
axon_growth_max_steps = 100

[[departments]]
name = "Dept-1"
config = "dept1.toml"

[[connections]]
id = "conn_1"
from = "Dept-1.Shard-A.Socket-Out"
to = "Dept-1.Shard-B.Socket-In"
"#;

const VALID_DEPARTMENT_TOML: &str = r#"
[[shards]]
name = "Shard-A"
config = "sharda.toml"

[[connections]]
id = "conn_2"
from = "Shard-A.Socket-Out"
to = "Shard-B.Socket-In"
"#;

const VALID_SHARD_TOML: &str = r#"
[dimensions]
w = 500
d = 500
h = 100

[settings]
ghost_capacity = 10
prune_threshold = -100
max_sprouts = 5
night_interval_ticks = 100
save_checkpoints_interval_ticks = 1000

[[layers]]
name = "Layer-1"
height_pct = 1.0
density = 0.5

[[layers.composition]]
type_name = "Neuron-A"
share = 1.0

[[neuron_types]]
name = "Neuron-A"

[neuron_types.membrane]
threshold = 1500
rest_potential = 0
leak_shift = 4
ahp_amplitude = 100

[neuron_types.timing]
refractory_period = 2
synapse_refractory_period = 3

[neuron_types.signal]
signal_propagation_length = 5

[neuron_types.homeostasis]
homeostasis_penalty = 50
homeostasis_decay = 10

[neuron_types.adaptive_leak]
adaptive_leak_min_shift = 0
adaptive_leak_gain = 5
adaptive_mode = 1

[neuron_types.dopamine]
d1_affinity = 10
d2_affinity = 20

[neuron_types.gsop]
gsop_potentiation = 10
gsop_depression = 5
initial_synapse_weight = 1000
is_inhibitory = false
inertia_curve = [1, 2, 3, 4, 5, 6, 7, 8]

[neuron_types.growth]
steering_fov_deg = 45.0
steering_radius_um = 100.0
steering_weight_inertia = 0.5
steering_weight_sensor = 0.3
steering_weight_jitter = 0.2
dendrite_radius_um = 10.0
growth_vertical_bias = 0.0
type_affinity = 1.0
dendrite_whitelist = ["Neuron-A"]
sprouting_weight_distance = 1.0
sprouting_weight_power = 2.0
sprouting_weight_explore = 0.5
sprouting_weight_type = 1.0

[neuron_types.spontaneous]
spontaneous_firing_period_ticks = 0

[[sockets]]
name = "Socket-In"
direction = "in"
width = 10
height = 10
entry_z = "Mid"
target_type = "Neuron-A"
growth_steps = 100

[[ports]]
name = "Port-Out"
direction = "out"
entry_z = "Top"

[[ports.pins]]
name = "Pin-1"
width = 1
height = 1
local_u = 0.5
local_v = 0.5
u_width = 0.1
v_height = 0.1
target_type = "Neuron-A"
stride = 1
growth_steps = 50
"#;

// 1. Valid minimal configs parse
#[test]
fn test_valid_minimal_configs_parse() {
    let model = parse_model_str(VALID_MODEL_TOML).unwrap();
    validate_model(&model).unwrap();

    let dept = parse_department_str(VALID_DEPARTMENT_TOML).unwrap();
    validate_department(&dept).unwrap();

    let shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    validate_shard(&shard).unwrap();
}

// 2. Unknown field rejected
#[test]
fn test_unknown_field_rejected() {
    let bad_model = format!("{}\nunknown_field = 42\n", VALID_MODEL_TOML);
    assert!(parse_model_str(&bad_model).is_err());
}

// 3. Duplicate names rejected
#[test]
fn test_duplicate_names_rejected() {
    // Duplicate department in model
    let bad_model = format!(
        "{}\n[[departments]]\nname = \"Dept-1\"\nconfig = \"dup.toml\"\n",
        VALID_MODEL_TOML
    );
    let m = parse_model_str(&bad_model).unwrap();
    assert!(validate_model(&m).is_err());

    // Duplicate neuron type in shard
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    let dup_nt = shard.neuron_types[0].clone();
    shard.neuron_types.push(dup_nt);
    assert!(validate_shard(&shard).is_err());
}

// 4. Entity name regex validation
#[test]
fn test_entity_name_regex_validation() {
    let mut model = parse_model_str(VALID_MODEL_TOML).unwrap();
    model.departments[0].name = "Dept.Invalid".to_string();
    assert!(validate_model(&model).is_err());

    model.departments[0].name = "".to_string();
    assert!(validate_model(&model).is_err());
}

// 5. Bad endpoint path rejected
#[test]
fn test_bad_endpoint_path_rejected() {
    let mut model = parse_model_str(VALID_MODEL_TOML).unwrap();
    model.connections[0].from = "Dept1.ShardA".to_string(); // only 2 parts
    assert!(validate_model(&model).is_err());

    let mut dept = parse_department_str(VALID_DEPARTMENT_TOML).unwrap();
    dept.connections[0].from = "ShardA".to_string(); // only 1 part
    assert!(validate_department(&dept).is_err());
}

// 6. Too many neuron types rejected
#[test]
fn test_too_many_neuron_types_rejected() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    let base_nt = shard.neuron_types[0].clone();
    shard.neuron_types.clear();
    for i in 0..17 {
        let mut nt = base_nt.clone();
        nt.name = format!("N-{}", i);
        shard.neuron_types.push(nt);
    }
    assert!(validate_shard(&shard).is_err());
}

// 7. Missing referenced type rejected
#[test]
fn test_missing_referenced_type_rejected() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.layers[0].composition[0].type_name = "NonExistent".to_string();
    assert!(validate_shard(&shard).is_err());
}

// 8. Bad dimensions rejected
#[test]
fn test_bad_dimensions_rejected() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.dimensions.w = 1024;
    assert!(validate_shard(&shard).is_err());

    shard.dimensions.w = 500;
    shard.dimensions.h = 256;
    assert!(validate_shard(&shard).is_err());
}

// 9. Bad layer height sum rejected
#[test]
fn test_bad_layer_height_sum_rejected() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.layers[0].height_pct = 0.99;
    assert!(validate_shard(&shard).is_err());
}

// 10. Bad composition sum rejected
#[test]
fn test_bad_composition_sum_rejected() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.layers[0].composition[0].share = 0.8;
    assert!(validate_shard(&shard).is_err());
}

// 11. Fractional v_seg rejected
#[test]
fn test_fractional_v_seg_rejected() {
    let mut model = parse_model_str(VALID_MODEL_TOML).unwrap();
    // Use values that yield fractional v_seg
    model.simulation.signal_speed_m_s = 2.13;
    model.simulation.voxel_size_um = 2.0;
    model.simulation.segment_length_voxels = 10;
    model.simulation.tick_duration_us = 100;
    // v_seg_float = 2.13 * 100 / 20 = 10.65 (not integer)
    assert!(validate_model(&model).is_err());
}

// 12. Spontaneous firing period validation
#[test]
fn test_spontaneous_firing_period_validation() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.neuron_types[0]
        .spontaneous
        .spontaneous_firing_period_ticks = 1;
    assert!(validate_shard(&shard).is_err());

    shard.neuron_types[0]
        .spontaneous
        .spontaneous_firing_period_ticks = 2;
    assert!(validate_shard(&shard).is_ok());

    shard.neuron_types[0]
        .spontaneous
        .spontaneous_firing_period_ticks = 0;
    assert!(validate_shard(&shard).is_ok());
}

// 13. Axon growth max steps limit
#[test]
fn test_axon_growth_max_steps_limit() {
    let mut model = parse_model_str(VALID_MODEL_TOML).unwrap();
    model.simulation.axon_growth_max_steps = 256;
    assert!(validate_model(&model).is_err());
}

// 14. Pin UV overflow rejected
#[test]
fn test_pin_uv_overflow_rejected() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.ports.as_mut().unwrap()[0].pins[0].local_u = 0.95;
    shard.ports.as_mut().unwrap()[0].pins[0].u_width = 0.1;
    assert!(validate_shard(&shard).is_err());
}

// 15. Input socket with ghost_capacity 0 rejected
#[test]
fn test_input_socket_zero_ghost_capacity_rejected() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.settings.ghost_capacity = 0;
    assert!(validate_shard(&shard).is_err());
}

// 16. Serde u8 range rejection
#[test]
fn test_serde_u8_range_rejection() {
    // Timing refractory period > 255
    let bad_shard_toml =
        VALID_SHARD_TOML.replace("refractory_period = 2", "refractory_period = 256");
    assert!(parse_shard_str(&bad_shard_toml).is_err());
}

// 17. Initial_synapse_weight validation
#[test]
fn test_initial_synapse_weight_validation() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.neuron_types[0].gsop.initial_synapse_weight = 32653;
    assert!(validate_shard(&shard).is_ok());

    shard.neuron_types[0].gsop.initial_synapse_weight = 32654;
    assert!(validate_shard(&shard).is_err());
}

// 18. Density out of bounds
#[test]
fn test_density_out_of_bounds() {
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.layers[0].density = 1.1;
    assert!(validate_shard(&shard).is_err());
}

// 19. NaN/Inf Float values rejected
#[test]
fn test_nan_float_values_rejected() {
    // 1. world.width_um = NaN
    let mut model = parse_model_str(VALID_MODEL_TOML).unwrap();
    model.world.width_um = f64::NAN;
    assert!(validate_model(&model).is_err());

    // 2. layer.height_pct = NaN
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.layers[0].height_pct = f32::NAN;
    assert!(validate_shard(&shard).is_err());

    // 3. composition.share = NaN
    let mut shard2 = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard2.layers[0].composition[0].share = f32::NAN;
    assert!(validate_shard(&shard2).is_err());

    // 4. pin.u_width = NaN
    let mut shard3 = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard3.ports.as_mut().unwrap()[0].pins[0].u_width = f32::NAN;
    assert!(validate_shard(&shard3).is_err());
}

// Additional helper loader tests
#[test]
fn test_loaders_success_and_error() {
    let dir = std::env::temp_dir();
    let unique_name = format!(
        "temp_model_{}_{}.toml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let file_path = dir.join(unique_name);

    {
        let mut file = File::create(&file_path).unwrap();
        file.write_all(VALID_MODEL_TOML.as_bytes()).unwrap();
    }

    let loaded = load_model_from_file(&file_path).unwrap();
    assert_eq!(loaded.world.width_um, 1000.0);

    // best-effort cleanup
    let _ = std::fs::remove_file(&file_path);

    // IoError on non-existent path
    let non_existent = dir.join("does_not_exist_123456789.toml");
    let err = load_model_from_file(&non_existent);
    assert!(matches!(err, Err(ConfigError::IoError(_))));
}

// Additional test: max_dendrites in TOML rejected as unknown field inside [simulation]
#[test]
fn test_max_dendrites_field_rejected() {
    let bad_model = VALID_MODEL_TOML.replace(
        "axon_growth_max_steps = 100",
        "axon_growth_max_steps = 100\nmax_dendrites = 128",
    );
    assert!(parse_model_str(&bad_model).is_err());
}

// 20. Growth parameters validation tests
#[test]
fn test_growth_params_validation() {
    // NaN in steering_weight_inertia rejected
    let mut shard = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard.neuron_types[0].growth.steering_weight_inertia = f32::NAN;
    assert!(validate_shard(&shard).is_err());

    // Inf in growth_vertical_bias rejected
    let mut shard2 = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard2.neuron_types[0].growth.growth_vertical_bias = f32::INFINITY;
    assert!(validate_shard(&shard2).is_err());

    // steering_fov_deg <= 0 rejected
    let mut shard3 = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard3.neuron_types[0].growth.steering_fov_deg = 0.0;
    assert!(validate_shard(&shard3).is_err());

    // steering_fov_deg > 180 rejected
    let mut shard4 = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard4.neuron_types[0].growth.steering_fov_deg = 180.1;
    assert!(validate_shard(&shard4).is_err());

    // steering_radius_um <= 0 rejected
    let mut shard5 = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard5.neuron_types[0].growth.steering_radius_um = -1.0;
    assert!(validate_shard(&shard5).is_err());

    // dendrite_radius_um <= 0 rejected
    let mut shard6 = parse_shard_str(VALID_SHARD_TOML).unwrap();
    shard6.neuron_types[0].growth.dendrite_radius_um = 0.0;
    assert!(validate_shard(&shard6).is_err());
}
