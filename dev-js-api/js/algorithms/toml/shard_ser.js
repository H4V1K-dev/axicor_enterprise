/**
 * @fileoverview Shard serializer for TOML engine.
 * Converts frontend Shard data to shard.toml structure.
 */

/**
 * Gets a default neuron type definition structure.
 * @param {string} name 
 * @returns {Object}
 */
export function getDefaultNeuronType(name) {
  const isInhibitory = name.toLowerCase().includes('inh') || name.toLowerCase().includes('basket');
  return {
    name: name,
    membrane: {
      threshold: 20000,
      rest_potential: -70000,
      leak_shift: 4,
      ahp_amplitude: 0
    },
    timings: {
      refractory_period: 5,
      synapse_refractory_period: 10
    },
    signal: {
      signal_propagation_length: 8
    },
    homeostasis: {
      homeostasis_penalty: 1500,
      homeostasis_decay: 990
    },
    adaptive_leak: {
      adaptive_leak_min_shift: -5,
      adaptive_leak_gain: 2,
      adaptive_mode: 1
    },
    dopamine: {
      d1_affinity: 80,
      d2_affinity: 20
    },
    gsop: {
      gsop_potentiation: 15,
      gsop_depression: 5,
      is_inhibitory: isInhibitory,
      inertia_curve: [10, 20, 30, 40, 50, 60, 70, 80]
    },
    growth: {
      steering_fov_deg: 60.0,
      steering_radius_um: 100.0,
      steering_weight_inertia: 0.6,
      steering_weight_sensor: 0.3,
      steering_weight_jitter: 0.1,
      dendrite_radius_um: 150.0,
      growth_vertical_bias: 0.7,
      type_affinity: 0.5,
      dendrite_whitelist: [],
      sprouting_weight_distance: 0.4,
      sprouting_weight_power: 0.4,
      sprouting_weight_explore: 0.1,
      sprouting_weight_type: 0.1
    },
    spontaneous: {
      spontaneous_firing_period_ticks: 10000
    }
  };
}

/**
 * Serializes a shard object to the shard.toml structure.
 * @param {Object} shard
 * @returns {Object}
 */
export function serializeShard(shard) {
  // 1. Dimensions
  const dimensions = {
    w: shard.size.w,
    d: shard.size.d,
    h: shard.size.h
  };

  // 2. Cortical Layers & Composition
  const layers = (shard.layers || []).map(layer => {
    // Find populations belonging to this layer
    const layerPops = (shard.populations || [])
      .filter(pop => pop[0] === layer.name)
      .map(pop => {
        // Extract type name from ref path, e.g. "bio/sensory/photoreceptor" -> "photoreceptor"
        const parts = pop[1].split('/');
        return parts[parts.length - 1];
      });

    // Compute share equally distributed
    const composition = [];
    if (layerPops.length > 0) {
      const share = parseFloat((1.0 / layerPops.length).toFixed(4));
      layerPops.forEach((typeName, idx) => {
        composition.push({
          type_name: typeName,
          // Make sure the sum is exactly 1.0
          share: idx === layerPops.length - 1 ? parseFloat((1.0 - share * (layerPops.length - 1)).toFixed(4)) : share
        });
      });
    }

    return {
      name: layer.name,
      height_pct: layer.height_pct,
      density: layer.density || 1.0,
      composition: composition
    };
  });

  // 3. Neuron types (lut)
  const uniqueTypes = new Set();
  (shard.populations || []).forEach(pop => {
    const parts = pop[1].split('/');
    uniqueTypes.add(parts[parts.length - 1]);
  });
  
  const neuron_types = Array.from(uniqueTypes).map(typeName => getDefaultNeuronType(typeName));

  // 4. Input Ports
  const inputs = (shard.input_ports || []).map(port => {
    return {
      name: port.name,
      entry_z: "Mid",
      pins: [{
        name: port.name + "_pin",
        width: port.width,
        height: port.height,
        local_u: 0.0,
        local_v: 0.0,
        u_width: 1.0,
        v_height: 1.0,
        target_type: "All",
        stride: 1,
        growth_steps: 1000,
        empty_pixel: "skip"
      }]
    };
  });

  // 5. Output Ports
  const outputs = (shard.output_ports || []).map(port => {
    return {
      name: port.name,
      pins: [{
        name: port.name + "_pin",
        width: port.width,
        height: port.height,
        local_u: 0.0,
        local_v: 0.0,
        u_width: 1.0,
        v_height: 1.0,
        target_type: "All",
        stride: 1
      }]
    };
  });

  // 6. Settings
  const settings = {
    ghost_capacity: 1024,
    prune_threshold: 15,
    max_sprouts: 4,
    night_interval_ticks: 10000,
    save_checkpoints_interval_ticks: 100000
  };

  return {
    dimensions,
    layers,
    neuron_types,
    inputs,
    outputs,
    settings
  };
}
