//! Crate topology integration tests.

use config::{
    LayerConfig, NeuronType, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings,
};
use std::collections::HashSet;
use topology::{
    AxonGrowthInput, AxonGrowthStopReason, PlacedSoma, SingleShardTopology,
    SingleShardTopologyInput, TopologyEngine, TopologyError,
};
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
            fatigue_capacity: 255,
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
        4,
        "Expected exactly 4 dependencies, found: {:?}",
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
    assert!(
        deps.contains(&"layout".to_string()),
        "Missing 'layout' dependency"
    );
    assert!(
        deps.contains(&"physics".to_string()),
        "Missing 'physics' dependency"
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

// ==========================================
// Local Axon Growth Tests
// ==========================================

#[test]
fn test_topology_layout_types_segment_limit_consistency() {
    // Assert that MAX_SEGMENTS_PER_AXON - 1 matches MAX_SEGMENT_OFFSET (256 - 1 == 255)
    assert_eq!(
        layout::MAX_SEGMENTS_PER_AXON - 1,
        types::MAX_SEGMENT_OFFSET as usize
    );
}

#[test]
fn test_topology_unknown_variant_id_rejected() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    // Construct single-shard topology with an invalid variant_id = 1
    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 1, // Invalid: neuron_types length is 1, valid index is 0 only
        position: types::PackedPosition::new(5, 5, 5, 1),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(123);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let result = TopologyEngine::grow_local_axons(&input);
    assert_eq!(
        result.err(),
        Some(TopologyError::UnknownNeuronType { variant_id: 1 })
    );
}

#[test]
fn test_topology_reproducible_growth_path() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    // Soma placed at (5, 5, 5)
    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(5, 5, 5, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(42);

    let input1 = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };
    let input2 = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res1 = TopologyEngine::grow_local_axons(&input1).unwrap();
    let res2 = TopologyEngine::grow_local_axons(&input2).unwrap();

    assert_eq!(res1.axons.len(), 1);
    assert_eq!(res2.axons.len(), 1);
    assert_eq!(res1.axons[0].segments, res2.axons[0].segments);
    assert_eq!(res1.axons[0].stop_reason, res2.axons[0].stop_reason);
}

#[test]
fn test_topology_growth_uses_source_variant_growth_params() {
    // We create TypeA with positive vertical bias (prefers going up Z)
    let mut type_a = make_dummy_neuron_type("TypeA");
    type_a.growth.growth_vertical_bias = 5.0; // Strong up bias
    type_a.growth.steering_weight_inertia = 0.0;
    type_a.growth.steering_weight_jitter = 0.0;

    // We create TypeB with negative vertical bias (prefers going down Z)
    let mut type_b = make_dummy_neuron_type("TypeB");
    type_b.growth.growth_vertical_bias = -5.0; // Strong down bias
    type_b.growth.steering_weight_inertia = 0.0;
    type_b.growth.steering_weight_jitter = 0.0;

    let neuron_types = vec![type_a, type_b];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    // Height=20
    let config = make_basic_test_config(10, 10, 20, layers, neuron_types);

    let somas = vec![
        PlacedSoma {
            soma_id: 0,
            variant_id: 0, // TypeA (goes up)
            position: types::PackedPosition::new(5, 5, 10, 0),
        },
        PlacedSoma {
            soma_id: 1,
            variant_id: 1, // TypeB (goes down)
            position: types::PackedPosition::new(5, 5, 10, 1),
        },
    ];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(99);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    assert_eq!(res.axons.len(), 2);

    // TypeA axon should grow upwards (Z increases)
    let axon_a = &res.axons[0];
    assert!(!axon_a.segments.is_empty());
    for seg in &axon_a.segments {
        assert!(seg.position.z() > 10);
    }

    // TypeB axon should grow downwards (Z decreases)
    let axon_b = &res.axons[1];
    assert!(!axon_b.segments.is_empty());
    for seg in &axon_b.segments {
        assert!(seg.position.z() < 10);
    }
}

#[test]
fn test_topology_segment_position_type_id_matches_source_variant() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(5, 5, 5, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(11);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    let axon = &res.axons[0];
    assert!(!axon.segments.is_empty());
    for seg in &axon.segments {
        assert_eq!(seg.position.type_id(), 0);
    }
}

#[test]
fn test_topology_source_origin_not_revisited() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(5, 5, 5, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(7);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    let axon = &res.axons[0];
    for seg in &axon.segments {
        let x = seg.position.x() as u32;
        let y = seg.position.y() as u32;
        let z = seg.position.z() as u32;
        assert!(
            x != 5 || y != 5 || z != 5,
            "Axon path revisited the source soma origin!"
        );
    }
}

#[test]
fn test_topology_boundary_stop_reason_after_selected_oob_direction() {
    // We force vertical bias strictly down and inertia = 0
    let mut type_a = make_dummy_neuron_type("TypeA");
    type_a.growth.growth_vertical_bias = -5.0; // Prefers Z decreases
    type_a.growth.steering_weight_inertia = 0.0;
    type_a.growth.steering_weight_jitter = 0.0;

    let neuron_types = vec![type_a];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    // Soma placed at Z=0. Since vertical_bias is negative, the highest score candidate
    // points to Z=-1, which is Out of Bounds. It must immediately trigger BoundaryReached stop reason.
    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(5, 5, 0, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(77);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    assert_eq!(res.axons.len(), 1);
    assert_eq!(
        res.axons[0].stop_reason,
        AxonGrowthStopReason::BoundaryReached
    );
    assert!(res.axons[0].segments.is_empty());
}

#[test]
fn test_topology_blocked_stop_reason() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    // 3x3x3 shard
    let config = make_basic_test_config(3, 3, 3, layers, neuron_types);

    // We place the target soma in the center (1, 1, 1).
    // And block all surrounding 26 voxels in 3x3x3 space with somas of other neurons.
    let mut somas = Vec::new();
    let mut soma_id = 0;

    // Target soma
    somas.push(PlacedSoma {
        soma_id,
        variant_id: 0,
        position: types::PackedPosition::new(1, 1, 1, 0),
    });
    soma_id += 1;

    // Obstacles
    for z in 0..3 {
        for y in 0..3 {
            for x in 0..3 {
                if x == 1 && y == 1 && z == 1 {
                    continue;
                }
                somas.push(PlacedSoma {
                    soma_id,
                    variant_id: 0,
                    position: types::PackedPosition::new(x, y, z, 0),
                });
                soma_id += 1;
            }
        }
    }

    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(999);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    // The target soma is at index 0
    let target_axon = &res.axons[0];
    assert_eq!(target_axon.soma_id, 0);
    assert_eq!(target_axon.stop_reason, AxonGrowthStopReason::Blocked);
    assert!(target_axon.segments.is_empty());
}

#[test]
fn test_topology_no_self_intersection() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(20, 20, 20, layers, neuron_types);

    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(10, 10, 10, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(100);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    let axon = &res.axons[0];
    let mut coords_set = HashSet::new();
    // Insert source soma voxel
    coords_set.insert((10, 10, 10));

    for seg in &axon.segments {
        let x = seg.position.x() as u32;
        let y = seg.position.y() as u32;
        let z = seg.position.z() as u32;
        assert!(coords_set.insert((x, y, z)), "Self-intersection detected!");
    }
}

#[test]
fn test_topology_growth_output_order_matches_somas() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    let somas = vec![
        PlacedSoma {
            soma_id: 3,
            variant_id: 0,
            position: types::PackedPosition::new(2, 2, 2, 0),
        },
        PlacedSoma {
            soma_id: 1,
            variant_id: 0,
            position: types::PackedPosition::new(5, 5, 5, 0),
        },
        PlacedSoma {
            soma_id: 2,
            variant_id: 0,
            position: types::PackedPosition::new(8, 8, 8, 0),
        },
    ];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(55);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    assert_eq!(res.axons.len(), 3);
    assert_eq!(res.axons[0].soma_id, 3);
    assert_eq!(res.axons[1].soma_id, 1);
    assert_eq!(res.axons[2].soma_id, 2);
}

#[test]
fn test_topology_fixed_point_steering_no_float_runtime() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(5, 5, 5, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(9999);

    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    // Just verify that execution succeeds without exceptions or runtime float failures.
    let res = TopologyEngine::grow_local_axons(&input);
    assert!(res.is_ok());
}

#[test]
fn test_topology_max_segment_length_respected() {
    let mut type_a = make_dummy_neuron_type("TypeA");
    type_a.growth.steering_weight_inertia = 0.0;
    type_a.growth.steering_weight_jitter = 1.0;
    type_a.growth.growth_vertical_bias = 0.0;
    let neuron_types = vec![type_a];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    // Large shard so the axon doesn't hit boundaries
    let config = make_basic_test_config(500, 500, 255, layers, neuron_types);
    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(250, 250, 127, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(12345);
    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    let axon = &res.axons[0];
    assert_eq!(axon.segments.len(), 255);
    assert_eq!(axon.stop_reason, AxonGrowthStopReason::MaxLengthReached);
}

#[test]
fn test_topology_segment_offset_contract() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(50, 50, 50, layers, neuron_types);
    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(25, 25, 25, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(7);
    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    let axon = &res.axons[0];
    assert!(!axon.segments.is_empty());
    for (i, seg) in axon.segments.iter().enumerate() {
        assert_eq!(seg.segment_offset, (i + 1) as u8);
    }
}

#[test]
fn test_topology_source_out_of_bounds_stop_reason() {
    let neuron_types = vec![make_dummy_neuron_type("TypeA")];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    // Max coordinates: 29x29x29
    let config = make_basic_test_config(30, 30, 30, layers, neuron_types);
    // Position at 40,40,40 is out of dimensions
    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(40, 40, 40, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(11);
    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input).unwrap();
    let axon = &res.axons[0];
    assert!(axon.segments.is_empty());
    assert_eq!(axon.stop_reason, AxonGrowthStopReason::SourceOutOfBounds);
}

#[test]
fn test_topology_invalid_growth_nan_rejected() {
    let mut type_a = make_dummy_neuron_type("TypeA");
    type_a.growth.steering_weight_inertia = f32::NAN;
    let neuron_types = vec![type_a];

    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);
    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(5, 5, 5, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(999);
    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input);
    assert_eq!(
        res.err(),
        Some(TopologyError::InvalidGrowthParameter {
            variant_id: 0,
            field: "steering_weight_inertia"
        })
    );
}

#[test]
fn test_topology_score_overflow_rejected() {
    let mut type_a = make_dummy_neuron_type("TypeA");
    // Large weights that fit in Q16 but overflow when summed:
    // 5.0e13 * 65536 = 3.27e18. Sum of three such terms overflows i64.
    type_a.growth.steering_weight_inertia = 5.0e13;
    type_a.growth.growth_vertical_bias = 5.0e13;
    type_a.growth.steering_weight_jitter = 5.0e13;
    let neuron_types = vec![type_a];

    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);
    let somas = vec![PlacedSoma {
        soma_id: 0,
        variant_id: 0,
        position: types::PackedPosition::new(5, 5, 5, 0),
    }];
    let topology = SingleShardTopology { somas };
    let seed = MasterSeed(999);
    let input = AxonGrowthInput {
        config: &config,
        topology: &topology,
        seed,
    };

    let res = TopologyEngine::grow_local_axons(&input);
    assert_eq!(res.err(), Some(TopologyError::CapacityOverflow));
}

// ==========================================
// Stage B2: Local Synapse Formation Tests
// ==========================================

use topology::{AxonSegment, GrownAxonPath, LocalGrowthResult, SynapseFormationInput};

fn make_formation_test_setup() -> (ShardConfig, SingleShardTopology, LocalGrowthResult) {
    let neuron_types = vec![
        make_dummy_neuron_type("TypeA"),
        make_dummy_neuron_type("TypeB"),
    ];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.1,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(10, 10, 10, layers, neuron_types);

    let somas = vec![
        PlacedSoma {
            soma_id: 0,
            variant_id: 0,
            position: types::PackedPosition::new(3, 3, 3, 0),
        },
        PlacedSoma {
            soma_id: 1,
            variant_id: 1,
            position: types::PackedPosition::new(6, 6, 6, 1),
        },
    ];
    let topology = SingleShardTopology { somas };

    // Axon of soma 0 grows to 5,5,5. Axon of soma 1 grows to 4,4,4
    let axons = vec![
        GrownAxonPath {
            soma_id: 0,
            segments: vec![
                AxonSegment {
                    position: types::PackedPosition::new(4, 4, 4, 0),
                    segment_offset: 1,
                },
                AxonSegment {
                    position: types::PackedPosition::new(5, 5, 5, 0),
                    segment_offset: 2,
                },
            ],
            stop_reason: AxonGrowthStopReason::Blocked,
        },
        GrownAxonPath {
            soma_id: 1,
            segments: vec![
                AxonSegment {
                    position: types::PackedPosition::new(5, 5, 5, 1),
                    segment_offset: 1,
                },
                AxonSegment {
                    position: types::PackedPosition::new(4, 4, 4, 1),
                    segment_offset: 2,
                },
            ],
            stop_reason: AxonGrowthStopReason::Blocked,
        },
    ];
    let growth = LocalGrowthResult { axons };

    (config, topology, growth)
}

#[test]
fn test_topology_formation_deterministic_same_seed() {
    let (config, topology, growth) = make_formation_test_setup();
    let seed = MasterSeed(42);

    let input1 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed,
    };
    let input2 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed,
    };

    let res1 = TopologyEngine::form_local_synapses(&input1).unwrap();
    let res2 = TopologyEngine::form_local_synapses(&input2).unwrap();

    assert_eq!(res1, res2);
}

#[test]
fn test_topology_formation_whitelist_respected() {
    let (mut config, topology, growth) = make_formation_test_setup();
    // Target TypeB (soma 1) whitelists only "TypeX" (soma 0 is "TypeA", so should be blocked)
    config.neuron_types[1].growth.dendrite_whitelist = vec!["TypeX".to_string()];

    let input = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };

    let plan = TopologyEngine::form_local_synapses(&input).unwrap();
    // Soma 1 has variant_id 1, TypeB. It shouldn't receive any synapses from soma 0 (TypeA).
    let row1 = &plan.rows[1];
    assert!(row1.slots.is_empty());
}

#[test]
fn test_topology_formation_radius_boundary() {
    let (mut config, topology, _) = make_formation_test_setup();

    // Only one segment at (4,4,4) for axon 0, so distance to soma 1 (6,6,6) is exactly dx=2, dy=2, dz=2 -> dist_sq = 12.
    let growth = LocalGrowthResult {
        axons: vec![
            GrownAxonPath {
                soma_id: 0,
                segments: vec![AxonSegment {
                    position: types::PackedPosition::new(4, 4, 4, 0),
                    segment_offset: 1,
                }],
                stop_reason: AxonGrowthStopReason::Blocked,
            },
            GrownAxonPath {
                soma_id: 1,
                segments: vec![],
                stop_reason: AxonGrowthStopReason::Blocked,
            },
        ],
    };

    // voxel_size_um = 1.0. If radius is 3.4 um, radius_voxels = ceil(3.4) = 4, radius_voxels_sq = 16. Candidate allowed.
    // If radius is 3.0 um, radius_voxels = ceil(3.0) = 3, radius_voxels_sq = 9. Candidate blocked.
    config.neuron_types[1].growth.dendrite_radius_um = 3.4;

    let input1 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };
    let plan1 = TopologyEngine::form_local_synapses(&input1).unwrap();
    assert!(!plan1.rows[1].slots.is_empty());

    config.neuron_types[1].growth.dendrite_radius_um = 3.0;
    let input2 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };
    let plan2 = TopologyEngine::form_local_synapses(&input2).unwrap();
    assert!(plan2.rows[1].slots.is_empty());
}

#[test]
fn test_topology_formation_max_dendrites_cap() {
    let (mut config, topology, mut growth) = make_formation_test_setup();
    // Make target radius huge so it sees all segments
    config.neuron_types[1].growth.dendrite_radius_um = 100.0;

    // Create axon path with 200 segments for soma 0
    let mut segments = Vec::new();
    for i in 1..=200 {
        segments.push(AxonSegment {
            position: types::PackedPosition::new(5, 5, 5, 0),
            segment_offset: i as u8,
        });
    }
    growth.axons[0].segments = segments;

    let input = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };

    let plan = TopologyEngine::form_local_synapses(&input).unwrap();
    let row1 = &plan.rows[1]; // Target is soma 1
    assert_eq!(row1.slots.len(), 128); // Capped to MAX_DENDRITES
    assert_eq!(plan.dropped_candidates, 72); // 200 - 128 = 72
}

#[test]
fn test_topology_formation_packed_target_safety() {
    let (config, topology, mut growth) = make_formation_test_setup();

    // Set segment offset to 0 (which is invalid)
    growth.axons[0].segments[0].segment_offset = 0;

    let input = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };

    let res = TopologyEngine::form_local_synapses(&input);
    assert_eq!(
        res.err(),
        Some(TopologyError::InvalidSynapseTarget {
            axon_id: 0,
            segment_offset: 0
        })
    );
}

#[test]
fn test_topology_formation_no_self_synapse() {
    let (config, topology, growth) = make_formation_test_setup();

    let input = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };

    let plan = TopologyEngine::form_local_synapses(&input).unwrap();
    // Soma 0 has axon segments. It shouldn't form synapses on itself (soma 0)
    let row0 = &plan.rows[0];
    assert!(row0.slots.iter().all(|s| s.source_soma_id != 0));
}

#[test]
fn test_topology_formation_initial_weights() {
    let (mut config, topology, growth) = make_formation_test_setup();

    // Case 1: normal weights
    config.neuron_types[0].gsop.initial_synapse_weight = 100;
    config.neuron_types[0].gsop.is_inhibitory = true; // GABA -> negative weight
    config.neuron_types[1].gsop.initial_synapse_weight = 200;
    config.neuron_types[1].gsop.is_inhibitory = false; // Glutamate -> positive weight

    let input1 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };
    let plan1 = TopologyEngine::form_local_synapses(&input1).unwrap();
    // Target 1 receives synapse from source 0 (inhibitory, weight = -100 << 16)
    let w0 = plan1.rows[1].slots[0].weight;
    assert_eq!(w0, -(100 << 16));
    // Verify weight_to_charge roundtrip
    assert_eq!(physics::weight_to_charge(w0), -100);

    // Target 0 receives synapse from source 1 (excitatory, weight = 200 << 16)
    let w1 = plan1.rows[0].slots[0].weight;
    assert_eq!(w1, 200 << 16);
    assert_eq!(physics::weight_to_charge(w1), 200);

    // Case 2: initial weight is 0 -> corrected to MIN_WEIGHT_LIMIT (1)
    config.neuron_types[0].gsop.initial_synapse_weight = 0;
    config.neuron_types[0].gsop.is_inhibitory = true;
    let input2 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };
    let plan2 = TopologyEngine::form_local_synapses(&input2).unwrap();
    assert_eq!(plan2.rows[1].slots[0].weight, -1);
}

#[test]
fn test_topology_formation_empty_slot_policy() {
    let (config, topology, growth) = make_formation_test_setup();

    let input = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };

    let plan = TopologyEngine::form_local_synapses(&input).unwrap();

    // The DTO plan should only have live synapses. No empty/tombstone target IDs.
    for row in &plan.rows {
        for slot in &row.slots {
            assert!(!slot.target.is_zero_none());
            assert!(!slot.target.is_tombstone());
            assert!(slot.target.is_valid_raw());
            assert!(!slot.target.is_inactive());
        }
    }
}

#[test]
fn test_topology_formation_invalid_voxel_size_rejected() {
    let (config, topology, growth) = make_formation_test_setup();

    let input1 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: f32::NAN,
        seed: MasterSeed(42),
    };
    assert_eq!(
        TopologyEngine::form_local_synapses(&input1).err(),
        Some(TopologyError::InvalidGrowthParameter {
            variant_id: 0,
            field: "voxel_size_um"
        })
    );

    let input2 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 0.0,
        seed: MasterSeed(42),
    };
    assert_eq!(
        TopologyEngine::form_local_synapses(&input2).err(),
        Some(TopologyError::InvalidGrowthParameter {
            variant_id: 0,
            field: "voxel_size_um"
        })
    );

    let input3 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: -2.5,
        seed: MasterSeed(42),
    };
    assert_eq!(
        TopologyEngine::form_local_synapses(&input3).err(),
        Some(TopologyError::InvalidGrowthParameter {
            variant_id: 0,
            field: "voxel_size_um"
        })
    );

    let input4 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: f32::INFINITY,
        seed: MasterSeed(42),
    };
    assert_eq!(
        TopologyEngine::form_local_synapses(&input4).err(),
        Some(TopologyError::InvalidGrowthParameter {
            variant_id: 0,
            field: "voxel_size_um"
        })
    );
}

#[test]
fn test_topology_formation_invalid_dendrite_radius_rejected() {
    let (mut config, topology, growth) = make_formation_test_setup();

    // Case 1: dendrite_radius_um = 0.0
    config.neuron_types[1].growth.dendrite_radius_um = 0.0;
    let input1 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };
    assert_eq!(
        TopologyEngine::form_local_synapses(&input1).err(),
        Some(TopologyError::InvalidGrowthParameter {
            variant_id: 1,
            field: "dendrite_radius_um"
        })
    );

    // Case 2: dendrite_radius_um = NaN
    config.neuron_types[1].growth.dendrite_radius_um = f32::NAN;
    let input2 = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };
    assert_eq!(
        TopologyEngine::form_local_synapses(&input2).err(),
        Some(TopologyError::InvalidGrowthParameter {
            variant_id: 1,
            field: "dendrite_radius_um"
        })
    );
}

#[test]
fn test_topology_formation_growth_topology_mismatch_rejected() {
    let (config, topology, mut growth) = make_formation_test_setup();

    // Change soma_id of the second axon to create a mismatch with topology.somas[1] (which is soma_id=1)
    growth.axons[1].soma_id = 999;

    let input = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };
    assert_eq!(
        TopologyEngine::form_local_synapses(&input).err(),
        Some(TopologyError::CapacityOverflow)
    );
}

#[test]
fn test_topology_formation_huge_radius_saturates() {
    let (mut config, topology, growth) = make_formation_test_setup();

    // Very large but finite radius
    config.neuron_types[1].growth.dendrite_radius_um = 1e20;

    let input = SynapseFormationInput {
        config: &config,
        topology: &topology,
        growth: &growth,
        voxel_size_um: 1.0,
        seed: MasterSeed(42),
    };

    let res = TopologyEngine::form_local_synapses(&input);
    assert!(res.is_ok());
    let plan = res.unwrap();
    // Shard is fully covered, so target 1 (TypeB) should receive synapses.
    assert!(!plan.rows[1].slots.is_empty());
}

#[test]
fn test_sprout_ranking_logic() {
    use core::cmp::Ordering;
    use topology::night_planning::{cmp_rank, SproutRankKey};

    let k1 = SproutRankKey {
        score_fixed: 1000,
        power_fixed: 50,
        target_soma_id: 10,
        dendrite_slot: 0,
    };
    // Higher score has priority (comes first/is greater, so cmp_rank (k2, k1) should be Ordering::Less)
    let k2 = SproutRankKey {
        score_fixed: 2000,
        power_fixed: 50,
        target_soma_id: 10,
        dendrite_slot: 0,
    };
    assert_eq!(cmp_rank(&k1, &k2), Ordering::Greater); // k1 < k2, so a < b returns Greater because we sort DESC on score

    // Same score, higher power DESC
    let k3 = SproutRankKey {
        score_fixed: 1000,
        power_fixed: 60,
        target_soma_id: 10,
        dendrite_slot: 0,
    };
    assert_eq!(cmp_rank(&k1, &k3), Ordering::Greater);

    // Same score/power, target_soma_id ASC
    let k4 = SproutRankKey {
        score_fixed: 1000,
        power_fixed: 50,
        target_soma_id: 5,
        dendrite_slot: 0,
    };
    assert_eq!(cmp_rank(&k1, &k4), Ordering::Greater); // k1 (soma 10) vs k4 (soma 5) -> 10 > 5 -> Greater

    // Same score/power/soma, slot ASC
    let k5 = SproutRankKey {
        score_fixed: 1000,
        power_fixed: 50,
        target_soma_id: 10,
        dendrite_slot: 1,
    };
    assert_eq!(cmp_rank(&k1, &k5), Ordering::Less); // k1 (slot 0) vs k5 (slot 1) -> 0 < 1 -> Less
}

#[test]
fn test_compute_power_fixed() {
    use topology::night_planning::compute_power_fixed;
    // 128 elements
    let mut weights = [0i32; 128];
    for w in weights.iter_mut() {
        *w = 128;
    }
    // average = 128 * 128 / 128 = 128. Clamp to 65535.
    assert_eq!(compute_power_fixed(&weights), 128);

    // Absolute values used
    for (i, w) in weights.iter_mut().enumerate() {
        if i % 2 == 0 {
            *w = -200;
        } else {
            *w = 200;
        }
    }
    // average = 200
    assert_eq!(compute_power_fixed(&weights), 200);

    // Clamp
    for w in weights.iter_mut() {
        *w = 100000;
    }
    // average = 100000. min(65535, 100000) = 65535
    assert_eq!(compute_power_fixed(&weights), 65535);
}

#[test]
fn test_sprout_scoring_fixed_point() {
    use topology::night_planning::{compute_sprout_score, SproutWeightParams};
    let params = SproutWeightParams {
        w_distance: 100,
        w_power: 50,
        w_explore: 10,
    };

    let score = compute_sprout_score(&params, 5, 20, 10).unwrap();
    // 10 * 5 + 100 * 20 + 50 * 10 = 50 + 2000 + 500 = 2550
    assert_eq!(score, 2550);

    // Overflow check
    let params_huge = SproutWeightParams {
        w_distance: u32::MAX,
        w_power: u32::MAX,
        w_explore: u32::MAX,
    };
    let score_overflow = compute_sprout_score(&params_huge, 65535, u32::MAX, 65535);
    assert!(score_overflow.is_none());
}

#[test]
fn test_compaction_plan_construction() {
    use topology::night_planning::build_compaction_plan;
    use types::PackedTarget;

    // targets array of size 128
    let mut targets = [PackedTarget::NONE; 128];
    // Populate some active/inactive targets
    targets[0] = PackedTarget::pack(1, 0); // Active
    targets[1] = PackedTarget::TOMBSTONE; // Inactive
    targets[2] = PackedTarget::pack(2, 0); // Active
    targets[3] = PackedTarget::NONE; // Inactive
    targets[4] = PackedTarget::pack(3, 0); // Active

    let plan = build_compaction_plan(&targets);

    // Moves should be:
    // index 2 -> 1
    // index 4 -> 2
    assert_eq!(plan.moves, vec![(2, 1), (4, 2)]);
    // Total active = 3. limit = 128. tail_clear_count = 128 - 3 = 125.
    assert_eq!(plan.tail_clear_count, 125);
}

#[test]
fn test_choose_dendrite_slot() {
    use topology::night_planning::choose_dendrite_slot;
    use types::PackedTarget;

    let mut targets = [PackedTarget::pack(1, 0); 128];
    // All active
    assert_eq!(choose_dendrite_slot(&targets), None);

    // Slot 5 inactive
    targets[5] = PackedTarget::NONE;
    assert_eq!(choose_dendrite_slot(&targets), Some(5));

    // Slot 2 tombstone (takes priority as first)
    targets[2] = PackedTarget::TOMBSTONE;
    assert_eq!(choose_dendrite_slot(&targets), Some(2));
}

#[test]
fn test_pruning_plan_construction() {
    use topology::night_planning::plan_pruning;

    let weights = [0, 5, 20, 2, -3, -15, 8];
    // Threshold 6. Abs weights: [0, 5, 20, 2, 3, 15, 8]
    // Weights to prune (w != 0 and abs(w) < 6): 5 (idx 1), 2 (idx 3), -3 (idx 4)
    let prune_plan = plan_pruning(&weights, 6);
    assert_eq!(prune_plan, vec![1, 3, 4]);
}

#[test]
fn test_plan_sprouts_goldens() {
    use topology::night_planning::{plan_sprouts, SproutWeightParams};
    use types::{MasterSeed, PackedTarget};

    let padded_n = 4;
    let total_axons = 4;
    let max_sprouts = 2;

    // Initialize mock targets and weights
    let mut targets = vec![PackedTarget::NONE; 128 * padded_n as usize];
    let mut weights = vec![0i32; 128 * padded_n as usize];

    // Some weights to calculate power_fixed
    weights[0] = 1000;
    weights[128] = 500;

    let params = SproutWeightParams {
        w_distance: 10,
        w_power: 5,
        w_explore: 100,
    };

    // empty paths_blob, so it uses our modulo mock coordinate emulation
    let paths_blob = vec![0u8; 0];

    // 1. Run with seed 42
    let plan1 = plan_sprouts(
        &paths_blob,
        &weights,
        &targets,
        padded_n,
        total_axons,
        &params,
        MasterSeed(42),
        1, // epoch
        0, // shard_id
        max_sprouts,
        None,
        None,
    );

    // Assert max_sprouts cap is respected
    assert!(plan1.len() <= max_sprouts as usize);

    // 2. Run with same parameters & seed -> must be identical
    let plan2 = plan_sprouts(
        &paths_blob,
        &weights,
        &targets,
        padded_n,
        total_axons,
        &params,
        MasterSeed(42),
        1, // epoch
        0, // shard_id
        max_sprouts,
        None,
        None,
    );
    assert_eq!(plan1, plan2, "Sprout plan must be deterministic for same seed");

    // 3. Run with different seed -> should produce different plan or ordering
    let plan3 = plan_sprouts(
        &paths_blob,
        &weights,
        &targets,
        padded_n,
        total_axons,
        &params,
        MasterSeed(999),
        1, // epoch
        0, // shard_id
        max_sprouts,
        None,
        None,
    );
    // (Note: with random explore factor w_explore=100, different seeds give different ranks)
    assert_ne!(plan1, plan3, "Sprout plan must vary under different MasterSeed");
}
