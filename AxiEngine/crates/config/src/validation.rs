//! Validation logic for AxiEngine configuration files.

use crate::dto::*;
use crate::error::ConfigError;
use std::collections::HashSet;

/// Helper to validate finite float strictly greater than zero.
fn is_positive_finite_f64(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

/// Helper to validate finite float strictly greater than zero.
fn is_positive_finite_f32(x: f32) -> bool {
    x.is_finite() && x > 0.0
}

/// Helper to validate finite float greater than or equal to zero.
fn is_non_negative_finite_f32(x: f32) -> bool {
    x.is_finite() && x >= 0.0
}

/// Helper to validate finite float within range `0.0..=1.0`.
fn is_unit_finite_f32(x: f32) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}

/// Helper to validate entity names against `^[a-zA-Z0-9_-]+$`.
fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Helper to validate connection endpoints path formats.
fn validate_endpoint_path(path: &str, expected_parts: usize) -> bool {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() != expected_parts {
        return false;
    }
    parts.iter().all(|part| is_valid_name(part))
}

/// Helper to check if a collection of strings has unique elements.
fn has_unique_names<'a>(names: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name) {
            return false;
        }
    }
    true
}

/// Validates the global model configuration (`model.toml`).
///
/// # Errors
/// Returns [`ConfigError::ValidationError`] if any of the rules specified in §7 are violated.
pub fn validate_model(config: &ModelConfig) -> Result<(), ConfigError> {
    // 1. World Config Validation
    if !is_positive_finite_f64(config.world.width_um)
        || !is_positive_finite_f64(config.world.depth_um)
        || !is_positive_finite_f64(config.world.height_um)
    {
        return Err(ConfigError::ValidationError(
            "World dimensions must be positive finite floats".to_string(),
        ));
    }

    // 2. Simulation Params Validation
    let sim = &config.simulation;
    if sim.tick_duration_us == 0 {
        return Err(ConfigError::ValidationError(
            "tick_duration_us must be > 0".to_string(),
        ));
    }
    if sim.master_seed.is_empty() {
        return Err(ConfigError::ValidationError(
            "master_seed must not be empty".to_string(),
        ));
    }
    if !is_positive_finite_f32(sim.voxel_size_um) {
        return Err(ConfigError::ValidationError(
            "voxel_size_um must be positive finite".to_string(),
        ));
    }
    if sim.segment_length_voxels == 0 {
        return Err(ConfigError::ValidationError(
            "segment_length_voxels must be > 0".to_string(),
        ));
    }
    if !is_positive_finite_f32(sim.signal_speed_m_s) {
        return Err(ConfigError::ValidationError(
            "signal_speed_m_s must be positive finite".to_string(),
        ));
    }
    if sim.sync_batch_ticks == 0 {
        return Err(ConfigError::ValidationError(
            "sync_batch_ticks must be > 0".to_string(),
        ));
    }
    if sim.axon_growth_max_steps > 255 {
        return Err(ConfigError::ValidationError(
            "axon_growth_max_steps must be <= 255".to_string(),
        ));
    }

    // Call physics::compute_v_seg
    physics::compute_v_seg(
        sim.signal_speed_m_s,
        sim.tick_duration_us,
        sim.voxel_size_um,
        sim.segment_length_voxels,
    )
    .map_err(|e| {
        ConfigError::ValidationError(format!(
            "Discrete segment velocity validation failed: {:?}",
            e
        ))
    })?;

    // 3. Departments Validation
    let dept_names: Vec<&str> = config.departments.iter().map(|d| d.name.as_str()).collect();
    if !has_unique_names(dept_names.clone()) {
        return Err(ConfigError::ValidationError(
            "Department names must be unique".to_string(),
        ));
    }

    for dept in &config.departments {
        if !is_valid_name(&dept.name) {
            return Err(ConfigError::ValidationError(format!(
                "Department name '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                dept.name
            )));
        }
        if dept.config.is_empty() {
            return Err(ConfigError::ValidationError(format!(
                "Department '{}' config path must not be empty",
                dept.name
            )));
        }
    }

    // 4. Connections Validation
    let conn_ids: Vec<&str> = config.connections.iter().map(|c| c.id.as_str()).collect();
    if !has_unique_names(conn_ids) {
        return Err(ConfigError::ValidationError(
            "Connection IDs must be unique".to_string(),
        ));
    }

    for conn in &config.connections {
        if !is_valid_name(&conn.id) {
            return Err(ConfigError::ValidationError(format!(
                "Connection ID '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                conn.id
            )));
        }
        if !validate_endpoint_path(&conn.from, 3) {
            return Err(ConfigError::ValidationError(format!(
                "Connection 'from' path '{}' must have exactly 3 components (DeptName.ShardName.SocketName) matching regex",
                conn.from
            )));
        }
        if !validate_endpoint_path(&conn.to, 3) {
            return Err(ConfigError::ValidationError(format!(
                "Connection 'to' path '{}' must have exactly 3 components (DeptName.ShardName.SocketName) matching regex",
                conn.to
            )));
        }
    }

    Ok(())
}

/// Validates the department configuration (`department.toml`).
///
/// # Errors
/// Returns [`ConfigError::ValidationError`] if any of the rules specified in §9 are violated.
pub fn validate_department(config: &DepartmentConfig) -> Result<(), ConfigError> {
    // 1. Shards Validation
    let shard_names: Vec<&str> = config.shards.iter().map(|s| s.name.as_str()).collect();
    if !has_unique_names(shard_names) {
        return Err(ConfigError::ValidationError(
            "Shard names must be unique in department".to_string(),
        ));
    }

    for shard in &config.shards {
        if !is_valid_name(&shard.name) {
            return Err(ConfigError::ValidationError(format!(
                "Shard name '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                shard.name
            )));
        }
        if shard.config.is_empty() {
            return Err(ConfigError::ValidationError(format!(
                "Shard '{}' config path must not be empty",
                shard.name
            )));
        }
    }

    // 2. Connections Validation
    let conn_ids: Vec<&str> = config.connections.iter().map(|c| c.id.as_str()).collect();
    if !has_unique_names(conn_ids) {
        return Err(ConfigError::ValidationError(
            "Connection IDs must be unique in department".to_string(),
        ));
    }

    for conn in &config.connections {
        if !is_valid_name(&conn.id) {
            return Err(ConfigError::ValidationError(format!(
                "Connection ID '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                conn.id
            )));
        }
        if !validate_endpoint_path(&conn.from, 2) {
            return Err(ConfigError::ValidationError(format!(
                "Connection 'from' path '{}' must have exactly 2 components (ShardName.SocketName) matching regex",
                conn.from
            )));
        }
        if !validate_endpoint_path(&conn.to, 2) {
            return Err(ConfigError::ValidationError(format!(
                "Connection 'to' path '{}' must have exactly 2 components (ShardName.SocketName) matching regex",
                conn.to
            )));
        }
    }

    Ok(())
}

/// Validates the shard configuration (`shard.toml`).
///
/// # Errors
/// Returns [`ConfigError::ValidationError`] if any of the rules specified in §10-§12 are violated.
pub fn validate_shard(config: &ShardConfig) -> Result<(), ConfigError> {
    // 1. Shard Dimensions Validation
    let dim = &config.dimensions;
    if !(1..=1023).contains(&dim.w) {
        return Err(ConfigError::ValidationError(
            "dimensions.w must be in range 1..=1023".to_string(),
        ));
    }
    if !(1..=1023).contains(&dim.d) {
        return Err(ConfigError::ValidationError(
            "dimensions.d must be in range 1..=1023".to_string(),
        ));
    }
    if !(1..=255).contains(&dim.h) {
        return Err(ConfigError::ValidationError(
            "dimensions.h must be in range 1..=255".to_string(),
        ));
    }

    // 2. Neuron Types Limit and Validation
    if config.neuron_types.is_empty() || config.neuron_types.len() > 16 {
        return Err(ConfigError::ValidationError(
            "neuron_types length must be in range 1..=16".to_string(),
        ));
    }

    let nt_names: Vec<&str> = config
        .neuron_types
        .iter()
        .map(|n| n.name.as_str())
        .collect();
    if !has_unique_names(nt_names.clone()) {
        return Err(ConfigError::ValidationError(
            "Neuron type names must be unique".to_string(),
        ));
    }

    let nt_set: HashSet<&str> = nt_names.iter().copied().collect();

    for nt in &config.neuron_types {
        if !is_valid_name(&nt.name) {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type name '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                nt.name
            )));
        }

        // Timing
        if nt.timing.refractory_period == 0 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' refractory_period must be > 0",
                nt.name
            )));
        }

        // Signal
        if nt.signal.signal_propagation_length == 0 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' signal_propagation_length must be > 0",
                nt.name
            )));
        }

        // Propagation length vs Refractory period invariant
        if nt.signal.signal_propagation_length < nt.timing.refractory_period {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' signal_propagation_length ({}) must be >= refractory_period ({})",
                nt.name, nt.signal.signal_propagation_length, nt.timing.refractory_period
            )));
        }

        // Gsop Curve length
        if nt.gsop.inertia_curve.len() != 8 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' inertia_curve must contain exactly 8 elements",
                nt.name
            )));
        }

        // Synapse weight range validation
        if nt.gsop.initial_synapse_weight > 32767 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' initial_synapse_weight ({}) must be <= 32767",
                nt.name, nt.gsop.initial_synapse_weight
            )));
        }

        // Adaptive mode
        if nt.adaptive_leak.adaptive_mode > 2 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' adaptive_mode must be 0, 1 or 2",
                nt.name
            )));
        }

        // Spontaneous firing period ticks
        if nt.spontaneous.spontaneous_firing_period_ticks == 1 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' spontaneous_firing_period_ticks cannot be 1 (0 to disable, >= 2 to enable)",
                nt.name
            )));
        }

        // Whitelist checks
        for target in &nt.growth.dendrite_whitelist {
            if !nt_set.contains(target.as_str()) {
                return Err(ConfigError::ValidationError(format!(
                    "Neuron type '{}' growth.dendrite_whitelist references non-existent neuron type '{}'",
                    nt.name, target
                )));
            }
        }
    }

    // 3. Anatomical Layers Validation
    if config.layers.is_empty() {
        return Err(ConfigError::ValidationError(
            "layers must not be empty".to_string(),
        ));
    }

    let layer_names: Vec<&str> = config.layers.iter().map(|l| l.name.as_str()).collect();
    if !has_unique_names(layer_names) {
        return Err(ConfigError::ValidationError(
            "Layer names must be unique".to_string(),
        ));
    }

    let mut total_height_pct = 0.0f32;
    for layer in &config.layers {
        if !is_valid_name(&layer.name) {
            return Err(ConfigError::ValidationError(format!(
                "Layer name '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                layer.name
            )));
        }

        if !is_positive_finite_f32(layer.height_pct) {
            return Err(ConfigError::ValidationError(format!(
                "Layer '{}' height_pct must be positive finite",
                layer.name
            )));
        }
        total_height_pct += layer.height_pct;

        if !is_unit_finite_f32(layer.density) {
            return Err(ConfigError::ValidationError(format!(
                "Layer '{}' density must be unit finite (0.0..=1.0)",
                layer.name
            )));
        }

        if layer.composition.is_empty() {
            return Err(ConfigError::ValidationError(format!(
                "Layer '{}' composition must not be empty",
                layer.name
            )));
        }

        let mut total_share = 0.0f32;
        for dist in &layer.composition {
            if !is_non_negative_finite_f32(dist.share) {
                return Err(ConfigError::ValidationError(format!(
                    "Layer '{}' composition share for '{}' must be non-negative finite",
                    layer.name, dist.type_name
                )));
            }
            total_share += dist.share;

            if !nt_set.contains(dist.type_name.as_str()) {
                return Err(ConfigError::ValidationError(format!(
                    "Layer '{}' composition references non-existent neuron type '{}'",
                    layer.name, dist.type_name
                )));
            }
        }

        if !total_share.is_finite() || (total_share - 1.0).abs() > 1e-4 {
            return Err(ConfigError::ValidationError(format!(
                "Layer '{}' composition shares must sum to 1.0 (got {})",
                layer.name, total_share
            )));
        }
    }

    if !total_height_pct.is_finite() || (total_height_pct - 1.0).abs() > 1e-4 {
        return Err(ConfigError::ValidationError(format!(
            "Anatomical layers height_pct must sum to 1.0 (got {})",
            total_height_pct
        )));
    }

    // 4. Sockets Validation
    let mut inbound_socket_exists = false;
    if let Some(sockets) = &config.sockets {
        let socket_names: Vec<&str> = sockets.iter().map(|s| s.name.as_str()).collect();
        if !has_unique_names(socket_names) {
            return Err(ConfigError::ValidationError(
                "Socket names must be unique".to_string(),
            ));
        }

        for socket in sockets {
            if !is_valid_name(&socket.name) {
                return Err(ConfigError::ValidationError(format!(
                    "Socket name '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                    socket.name
                )));
            }
            if socket.width == 0 || socket.height == 0 {
                return Err(ConfigError::ValidationError(format!(
                    "Socket '{}' width and height must be > 0",
                    socket.name
                )));
            }
            if let Some(steps) = socket.growth_steps {
                if steps > 255 {
                    return Err(ConfigError::ValidationError(format!(
                        "Socket '{}' growth_steps must be <= 255",
                        socket.name
                    )));
                }
            }
            if let Some(ref_type) = &socket.target_type {
                if !nt_set.contains(ref_type.as_str()) {
                    return Err(ConfigError::ValidationError(format!(
                        "Socket '{}' target_type references non-existent neuron type '{}'",
                        socket.name, ref_type
                    )));
                }
            }
            if socket.direction == Direction::In {
                inbound_socket_exists = true;
            }
        }
    }

    // If there is an inbound socket, settings.ghost_capacity must be > 0
    if inbound_socket_exists && config.settings.ghost_capacity == 0 {
        return Err(ConfigError::ValidationError(
            "ghost_capacity must be > 0 when there is at least one inbound socket".to_string(),
        ));
    }

    // 5. Ports Validation
    if let Some(ports) = &config.ports {
        let port_names: Vec<&str> = ports.iter().map(|p| p.name.as_str()).collect();
        if !has_unique_names(port_names) {
            return Err(ConfigError::ValidationError(
                "Port names must be unique".to_string(),
            ));
        }

        for port in ports {
            if !is_valid_name(&port.name) {
                return Err(ConfigError::ValidationError(format!(
                    "Port name '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                    port.name
                )));
            }

            let pin_names: Vec<&str> = port.pins.iter().map(|p| p.name.as_str()).collect();
            if !has_unique_names(pin_names) {
                return Err(ConfigError::ValidationError(format!(
                    "Pin names within port '{}' must be unique",
                    port.name
                )));
            }

            for pin in &port.pins {
                if !is_valid_name(&pin.name) {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin name '{}' does not match target regex ^[a-zA-Z0-9_-]+$",
                        pin.name
                    )));
                }
                if pin.width == 0 || pin.height == 0 {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' width and height must be > 0",
                        pin.name
                    )));
                }
                if pin.stride == 0 {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' stride must be > 0",
                        pin.name
                    )));
                }
                if !is_unit_finite_f32(pin.local_u) || !is_unit_finite_f32(pin.local_v) {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' local_u and local_v must be unit finite (0.0..=1.0)",
                        pin.name
                    )));
                }
                if !is_positive_finite_f32(pin.u_width) || !is_positive_finite_f32(pin.v_height) {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' u_width and v_height must be positive finite",
                        pin.name
                    )));
                }
                // Precision-safe projection boundary verification
                let u_sum = pin.local_u + pin.u_width;
                if !u_sum.is_finite() || u_sum > 1.0 {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' local_u + u_width ({}) exceeds 1.0 limit or is not finite",
                        pin.name, u_sum
                    )));
                }
                let v_sum = pin.local_v + pin.v_height;
                if !v_sum.is_finite() || v_sum > 1.0 {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' local_v + v_height ({}) exceeds 1.0 limit or is not finite",
                        pin.name, v_sum
                    )));
                }
                if let Some(steps) = pin.growth_steps {
                    if steps > 255 {
                        return Err(ConfigError::ValidationError(format!(
                            "Pin '{}' growth_steps must be <= 255",
                            pin.name
                        )));
                    }
                }
                if !nt_set.contains(pin.target_type.as_str()) {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' target_type references non-existent neuron type '{}'",
                        pin.name, pin.target_type
                    )));
                }
            }
        }
    }

    Ok(())
}
