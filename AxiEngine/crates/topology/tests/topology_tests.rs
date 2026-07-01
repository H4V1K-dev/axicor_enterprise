//! Crate topology integration tests.

use config::{
    LayerConfig, NeuronType, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings,
};
use std::collections::HashSet;
use topology::{SingleShardTopologyInput, TopologyEngine, TopologyError};
use types::MasterSeed;

/// Helper function to create dummy membrane, timings and growth properties for tests.
fn make_dummy_neuron_type(name: &str) -> NeuronType {
    NeuronType {
        name: name.to_string(),
        membrane: config::MembraneParams {
            threshold: 1000,
            rest_potential: 0,
            leak_shift: 1,
            ahp_amplitude: 0,
        },
        timing: config::TimingParams {
            refractory_period: 2,
            synapse_refractory_period: 2,
        },
        signal: config::SignalParams {
            signal_propagation_length: 10,
        },
        homeostasis: config::HomeostasisParams {
            homeostasis_penalty: 0,
            homeostasis_decay: 10,
        },
        adaptive_leak: config::AdaptiveLeakParams {
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
        },
        dopamine: config::DopamineParams {
            d1_affinity: 0,
            d2_affinity: 0,
        },
        gsop: config::GsopParams {
            gsop_potentiation: 1,
            gsop_depression: 1,
            initial_synapse_weight: 1000,
            is_inhibitory: false,
            inertia_curve: vec![1, 1, 1, 1, 1, 1, 1, 1],
        },
        growth: config::GrowthParams {
            steering_fov_deg: 45.0,
            steering_radius_um: 10.0,
            steering_weight_inertia: 0.5,
            steering_weight_sensor: 0.5,
            steering_weight_jitter: 0.1,
            dendrite_radius_um: 5.0,
            growth_vertical_bias: 0.0,
            type_affinity: 1.0,
            dendrite_whitelist: vec![],
            sprouting_weight_distance: 1.0,
            sprouting_weight_power: 1.0,
            sprouting_weight_explore: 1.0,
            sprouting_weight_type: 1.0,
        },
        spontaneous: config::SpontaneousParams {
            spontaneous_firing_period_ticks: 0,
        },
    }
}

/// Helper function to construct a basic valid config for placement testing.
fn make_basic_test_config(
    width: u32,
    depth: u32,
    height: u32,
    layers: Vec<LayerConfig>,
    neuron_types: Vec<NeuronType>,
) -> ShardConfig {
    ShardConfig {
        meta: None,
        dimensions: ShardDimensions {
            w: width,
            d: depth,
            h: height,
        },
        settings: ShardSettings {
            ghost_capacity: 1024,
            prune_threshold: 0,
            max_sprouts: 8,
            night_interval_ticks: 100,
            save_checkpoints_interval_ticks: 1000,
        },
        layers,
        neuron_types,
        sockets: None,
        ports: None,
    }
}

// 1. Reproducibility of soma placement
#[test]
fn test_topology_reproducible_soma_placement() {
    let neuron_types = vec![
        make_dummy_neuron_type("TypeA"),
        make_dummy_neuron_type("TypeB"),
    ];
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 1.0,
        density: 0.3,
        composition: vec![
            NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 0.6,
            },
            NeuronTypeDistribution {
                type_name: "TypeB".to_string(),
                share: 0.4,
            },
        ],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);
    let seed = MasterSeed(12345);

    let input1 = SingleShardTopologyInput {
        config: &config,
        seed,
    };
    let input2 = SingleShardTopologyInput {
        config: &config,
        seed,
    };

    let res1 = TopologyEngine::generate_single_shard_topology(&input1).unwrap();
    let res2 = TopologyEngine::generate_single_shard_topology(&input2).unwrap();

    assert_eq!(res1.somas.len(), res2.somas.len());
    assert!(!res1.somas.is_empty());
    for (s1, s2) in res1.somas.iter().zip(res2.somas.iter()) {
        assert_eq!(s1.soma_id, s2.soma_id);
        assert_eq!(s1.variant_id, s2.variant_id);
        assert_eq!(s1.position, s2.position);
    }
}

// 2. Determinism of seed changes (shift positions but preserve counts)
#[test]
fn test_topology_seed_changes_placement_but_preserves_counts() {
    let neuron_types = vec![
        make_dummy_neuron_type("TypeA"),
        make_dummy_neuron_type("TypeB"),
    ];
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 1.0,
        density: 0.25,
        composition: vec![
            NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 0.5,
            },
            NeuronTypeDistribution {
                type_name: "TypeB".to_string(),
                share: 0.5,
            },
        ],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    let input1 = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(42),
    };
    let input2 = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(999),
    };

    let res1 = TopologyEngine::generate_single_shard_topology(&input1).unwrap();
    let res2 = TopologyEngine::generate_single_shard_topology(&input2).unwrap();

    assert_eq!(res1.somas.len(), res2.somas.len());

    // Positions should differ
    let mut match_count = 0;
    for (s1, s2) in res1.somas.iter().zip(res2.somas.iter()) {
        if s1.position == s2.position {
            match_count += 1;
        }
    }
    assert!(
        match_count < res1.somas.len(),
        "Seed change did not change placement positions!"
    );

    // Variant counts must be identical
    let count1_a = res1.somas.iter().filter(|s| s.variant_id == 0).count();
    let count1_b = res1.somas.iter().filter(|s| s.variant_id == 1).count();
    let count2_a = res2.somas.iter().filter(|s| s.variant_id == 0).count();
    let count2_b = res2.somas.iter().filter(|s| s.variant_id == 1).count();

    assert_eq!(count1_a, count2_a);
    assert_eq!(count1_b, count2_b);
}

// 3. Absence of voxel collisions
#[test]
fn test_topology_no_voxel_collisions() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 1.0,
        density: 0.5,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(8, 8, 8, layers, neuron_types);
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(888),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input).unwrap();
    let mut positions = HashSet::new();

    for soma in res.somas {
        let raw = soma.position.0;
        assert!(
            positions.insert(raw),
            "Collision detected at raw packed position: 0x{:08X}",
            raw
        );
    }
}

// 4. Checking that density layers soma count matches formula
#[test]
fn test_topology_layer_density_respected() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 1.0,
        density: 0.23, // 1000 * 0.23 = 230
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(777),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input).unwrap();
    assert_eq!(res.somas.len(), 230);
}

// 5. Largest-Remainder Hamilton method and tie-breaking test
#[test]
fn test_topology_composition_largest_remainder_deterministic() {
    let neuron_types = vec![
        make_dummy_neuron_type("TypeA"), // variant_id = 0
        make_dummy_neuron_type("TypeB"), // variant_id = 1
        make_dummy_neuron_type("TypeC"), // variant_id = 2
    ];
    // Ideal count is 10 somas total.
    // Composition shares:
    // TypeA: 0.35 -> ideal = 3.5 -> base = 3, remainder = 0.5
    // TypeB: 0.35 -> ideal = 3.5 -> base = 3, remainder = 0.5
    // TypeC: 0.30 -> ideal = 3.0 -> base = 3, remainder = 0.0
    // Sum of bases = 9, remaining = 1.
    // Tie-break: TypeA (variant 0) < TypeB (variant 1), so TypeA gets the extra soma.
    // Expected: TypeA = 4, TypeB = 3, TypeC = 3.
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 1.0,
        density: 0.1, // capacity = 100, 100 * 0.1 = 10 somas
        composition: vec![
            NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 0.35,
            },
            NeuronTypeDistribution {
                type_name: "TypeB".to_string(),
                share: 0.35,
            },
            NeuronTypeDistribution {
                type_name: "TypeC".to_string(),
                share: 0.30,
            },
        ],
    }];
    let config = make_basic_test_config(10, 10, 1, layers, neuron_types);
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(111),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input).unwrap();
    assert_eq!(res.somas.len(), 10);

    let count_a = res.somas.iter().filter(|s| s.variant_id == 0).count();
    let count_b = res.somas.iter().filter(|s| s.variant_id == 1).count();
    let count_c = res.somas.iter().filter(|s| s.variant_id == 2).count();

    assert_eq!(count_a, 4);
    assert_eq!(count_b, 3);
    assert_eq!(count_c, 3);
}

// 6. Verification of layers Z range boundaries
#[test]
fn test_topology_packed_position_bounds() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    // h=10.
    // Layer1: height_pct = 0.4 -> cumulative limits: 0.0 -> 0.4. z_start = 0, z_end = 4. Range: [0..4)
    // Layer2: height_pct = 0.6 -> cumulative limits: 0.4 -> 1.0. z_start = 4, z_end = 10. Range: [4..10)
    let layers = vec![
        LayerConfig {
            name: "Layer1".to_string(),
            height_pct: 0.4,
            density: 0.2, // 10 * 10 * 4 * 0.2 = 80 somas
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
        LayerConfig {
            name: "Layer2".to_string(),
            height_pct: 0.6,
            density: 0.2, // 10 * 10 * 6 * 0.2 = 120 somas
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
    ];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(222),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input).unwrap();
    assert_eq!(res.somas.len(), 200);

    // Somas should be ordered by layer_index first.
    // First 80 somas belong to Layer1, next 120 belong to Layer2.
    for (i, soma) in res.somas.iter().enumerate() {
        let z = soma.position.z();
        if i < 80 {
            assert!(z < 4, "Soma {} in Layer1 has invalid z={}", i, z);
        } else {
            assert!(
                (4..10).contains(&z),
                "Soma {} in Layer2 has invalid z={}",
                i,
                z
            );
        }
    }
}

// 7. Rejection of layers with 0 voxel height and positive density
#[test]
fn test_topology_zero_height_layer_rejected() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    // h=10.
    // Layer1: height_pct = 0.5 -> z_start = 0, z_end = 5.
    // Layer2: height_pct = 0.05 -> z_start = 5, z_end = floor(5.5) = 5 (height = 0 voxels!).
    // Layer3: height_pct = 0.45 -> z_start = 5, z_end = 10.
    let layers = vec![
        LayerConfig {
            name: "Layer1".to_string(),
            height_pct: 0.5,
            density: 0.1,
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
        LayerConfig {
            name: "Layer2".to_string(),
            height_pct: 0.05,
            density: 0.1, // Positive density on 0 voxel layer!
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
        LayerConfig {
            name: "Layer3".to_string(),
            height_pct: 0.45,
            density: 0.1,
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
    ];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(333),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        TopologyError::LayerGeometryError {
            layer_index: 1,
            msg: "Layer 'Layer2' has 0 Z-voxels height but positive density=0.1".to_string()
        }
    );
}

// 8. Full layer capacity fill (density = 1.0)
#[test]
fn test_topology_full_layer_capacity_fill() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 1.0,
        density: 1.0, // Fill 100%
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(4, 4, 2, layers, neuron_types); // Capacity = 4 * 4 * 2 = 32
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(444),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input).unwrap();
    assert_eq!(res.somas.len(), 32);

    // Verify all positions are unique and filled
    let mut positions = HashSet::new();
    for soma in res.somas {
        assert!(positions.insert(soma.position.0));
    }
}

// 8a. Defensive checking of capacity exceeded error
#[test]
fn test_topology_layer_capacity_exceeded_defensive() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    // We manually set density to 1.5 bypassing typical TOML config validations.
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 1.0,
        density: 1.5, // Tries to place 150% somas
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(4, 4, 1, layers, neuron_types); // capacity = 16
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(555),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        TopologyError::LayerCapacityExceeded {
            layer_index: 0,
            max_capacity: 16
        }
    );
}

// 9. Verification of deterministic collision probing
#[test]
fn test_topology_deterministic_collision_probing() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 1.0,
        density: 0.75, // places 3 somas on capacity 4
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(2, 2, 1, layers, neuron_types); // capacity = 4
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(666),
    };

    // Should succeed because 3 < 4 capacity
    let res = TopologyEngine::generate_single_shard_topology(&input).unwrap();
    assert_eq!(res.somas.len(), 3);
}

// 10. Stable soma ID ordering
#[test]
fn test_topology_stable_soma_id_ordering() {
    let neuron_types = vec![
        make_dummy_neuron_type("TypeA"), // variant_id = 0
        make_dummy_neuron_type("TypeB"), // variant_id = 1
    ];
    let layers = vec![
        LayerConfig {
            name: "Layer1".to_string(),
            height_pct: 0.5,
            density: 0.2, // places 10 somas
            composition: vec![
                NeuronTypeDistribution {
                    type_name: "TypeB".to_string(),
                    share: 0.5,
                },
                NeuronTypeDistribution {
                    type_name: "TypeA".to_string(),
                    share: 0.5,
                },
            ],
        },
        LayerConfig {
            name: "Layer2".to_string(),
            height_pct: 0.5,
            density: 0.2, // places 10 somas
            composition: vec![
                NeuronTypeDistribution {
                    type_name: "TypeA".to_string(),
                    share: 0.5,
                },
                NeuronTypeDistribution {
                    type_name: "TypeB".to_string(),
                    share: 0.5,
                },
            ],
        },
    ];
    let config = make_basic_test_config(10, 10, 2, layers, neuron_types); // capacity = 100, 50 per layer.
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(777),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input).unwrap();
    assert_eq!(res.somas.len(), 40);

    // Verify sequential IDs and sorted groups
    let mut last_layer_index = 0_usize;
    let mut last_variant_id = 0_u8;

    for (i, soma) in res.somas.iter().enumerate() {
        assert_eq!(soma.soma_id, i as u32);

        let z = soma.position.z();
        let layer_index = if z == 0 { 0 } else { 1 };
        let variant_id = soma.variant_id;

        // Ensure layer index only goes forward
        assert!(layer_index >= last_layer_index);

        if layer_index > last_layer_index {
            last_layer_index = layer_index;
            last_variant_id = variant_id;
        } else {
            // Within same layer, variant_id must only go forward
            assert!(variant_id >= last_variant_id);
            last_variant_id = variant_id;
        }

        // Double check packed position type cache
        assert_eq!(soma.position.type_id(), variant_id);
    }
}

// 11. Forbidden dependencies check
#[test]
fn test_topology_no_forbidden_dependencies() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml_path = std::path::Path::new(manifest_dir).join("Cargo.toml");
    let content = std::fs::read_to_string(cargo_toml_path).unwrap();

    // 1. Assert no forbidden dependencies in the entire file
    let forbidden = &[
        "rand",
        "rand_chacha",
        "glam",
        "compute",
        "compute-api",
        "compute-cpu",
        "compute-cuda",
        "wire",
        "ipc",
        "vfs",
    ];
    for dep in forbidden {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(suffix) = trimmed.strip_prefix(dep) {
                let first_char = suffix.trim().chars().next();
                if first_char == Some('=') {
                    panic!(
                        "Forbidden dependency '{}' found in Cargo.toml: {}",
                        dep, trimmed
                    );
                }
            }
        }
    }

    // 2. Assert only allowed production dependencies: types and config
    let mut in_dependencies = false;
    let mut deps = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if in_dependencies && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(pos) = trimmed.find('=') {
                let name = trimmed[..pos].trim();
                deps.push(name.to_string());
            }
        }
    }

    assert_eq!(
        deps.len(),
        2,
        "Expected exactly 2 dependencies, found: {:?}",
        deps
    );
    assert!(
        deps.contains(&"types".to_string()),
        "Missing 'types' dependency"
    );
    assert!(
        deps.contains(&"config".to_string()),
        "Missing 'config' dependency"
    );
}

// 12. Variant_id matches PackedPosition type_id cache
#[test]
fn test_topology_variant_id_matches_packed_position_type_id() {
    let neuron_types = vec![
        make_dummy_neuron_type("TypeA"), // variant_id = 0
        make_dummy_neuron_type("TypeB"), // variant_id = 1
    ];
    let layers = vec![LayerConfig {
        name: "Layer1".to_string(),
        height_pct: 0.5,
        density: 0.1,
        composition: vec![
            NeuronTypeDistribution {
                type_name: "TypeB".to_string(),
                share: 0.5,
            },
            NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 0.5,
            },
        ],
    }];
    let config = make_basic_test_config(10, 10, 2, layers, neuron_types);
    let input = SingleShardTopologyInput {
        config: &config,
        seed: MasterSeed(777),
    };

    let res = TopologyEngine::generate_single_shard_topology(&input).unwrap();
    assert!(!res.somas.is_empty());
    for soma in res.somas {
        assert_eq!(
            soma.variant_id,
            soma.position.type_id(),
            "Type ID mismatch for soma {}: variant_id={} vs position.type_id={}",
            soma.soma_id,
            soma.variant_id,
            soma.position.type_id()
        );
    }
}

// Helper for Z-to-layer testing (replicating specification slicing rules)
fn find_layer_index(z: u32, config: &ShardConfig) -> usize {
    let shard_h = config.dimensions.h;
    let mut cumulative_before = 0.0_f64;
    for (idx, layer) in config.layers.iter().enumerate() {
        let height_pct = layer.height_pct as f64;
        let cumulative_after = cumulative_before + height_pct;
        let z_start = (cumulative_before * shard_h as f64).floor() as u32;
        let mut z_end = (cumulative_after * shard_h as f64).floor() as u32;
        if idx == config.layers.len() - 1 {
            z_end = shard_h;
        }
        if z >= z_start && z < z_end {
            return idx;
        }
        cumulative_before = cumulative_after;
    }
    config.layers.len() - 1
}

#[test]
fn test_topology_z_range_layer_mapping() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    // h=16.
    // Layer1: height_pct = 0.25 -> [0..4)
    // Layer2: height_pct = 0.50 -> [4..12)
    // Layer3: height_pct = 0.25 -> [12..16)
    let layers = vec![
        LayerConfig {
            name: "L1".to_string(),
            height_pct: 0.25,
            density: 0.1,
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
        LayerConfig {
            name: "L2".to_string(),
            height_pct: 0.50,
            density: 0.1,
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
        LayerConfig {
            name: "L3".to_string(),
            height_pct: 0.25,
            density: 0.1,
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
    ];
    let config = make_basic_test_config(10, 10, 16, layers, neuron_types);

    // Test boundaries
    assert_eq!(find_layer_index(0, &config), 0);
    assert_eq!(find_layer_index(3, &config), 0);
    assert_eq!(find_layer_index(4, &config), 1);
    assert_eq!(find_layer_index(11, &config), 1);
    assert_eq!(find_layer_index(12, &config), 2);
    assert_eq!(find_layer_index(15, &config), 2);
}
