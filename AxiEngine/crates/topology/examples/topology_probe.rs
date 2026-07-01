// This is a diagnostic, dev-only tool for visualization and statistical analysis.
// Architectural definitions are defined in topology_spec.md and topology source code.
// This example is not a production API.

use config::{
    LayerConfig, NeuronType, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings,
};
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::io::Write;
use topology::{SingleShardTopologyInput, TopologyEngine};
use types::MasterSeed;

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

/// Recovers the layer index from a given Z coordinate using layer slicing rules.
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

fn main() {
    println!("=== Topology Diagnostic Probe ===");

    // 1. Manually configure a shard config for probe visualization
    let neuron_types = vec![
        make_dummy_neuron_type("TypeA"), // variant_id = 0
        make_dummy_neuron_type("TypeB"), // variant_id = 1
        make_dummy_neuron_type("TypeC"), // variant_id = 2
    ];

    let layers = vec![
        LayerConfig {
            name: "Layer_Bottom".to_string(),
            height_pct: 0.25, // Height: Z in [0..4)
            density: 0.10,    // 32 * 32 * 4 * 0.1 = 409 somas
            composition: vec![NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        },
        LayerConfig {
            name: "Layer_Middle".to_string(),
            height_pct: 0.50, // Height: Z in [4..12)
            density: 0.05,    // 32 * 32 * 8 * 0.05 = 409 somas
            composition: vec![
                NeuronTypeDistribution {
                    type_name: "TypeB".to_string(),
                    share: 0.5,
                },
                NeuronTypeDistribution {
                    type_name: "TypeC".to_string(),
                    share: 0.5,
                },
            ],
        },
        LayerConfig {
            name: "Layer_Top".to_string(),
            height_pct: 0.25, // Height: Z in [12..16)
            density: 0.20,    // 32 * 32 * 4 * 0.2 = 819 somas
            composition: vec![
                NeuronTypeDistribution {
                    type_name: "TypeA".to_string(),
                    share: 0.3,
                },
                NeuronTypeDistribution {
                    type_name: "TypeC".to_string(),
                    share: 0.7,
                },
            ],
        },
    ];

    let config = ShardConfig {
        meta: None,
        dimensions: ShardDimensions {
            w: 32,
            d: 32,
            h: 16,
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
    };

    let seed = MasterSeed(987654321);
    let input = SingleShardTopologyInput {
        config: &config,
        seed,
    };

    // 2. Generate topology
    let result = match TopologyEngine::generate_single_shard_topology(&input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to generate topology: {:?}", e);
            std::process::exit(1);
        }
    };

    let total_somas = result.somas.len();
    let shard_capacity =
        (config.dimensions.w as u64) * (config.dimensions.d as u64) * (config.dimensions.h as u64);
    let occupancy_ratio = total_somas as f64 / shard_capacity as f64;

    // 3. Setup artifacts directory
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let artifacts_dir = std::path::Path::new(manifest_dir).join("../../artifacts");
    if let Err(e) = create_dir_all(&artifacts_dir) {
        eprintln!("Failed to create artifacts directory: {}", e);
        std::process::exit(1);
    }

    // 4. Write somas CSV
    let somas_csv_path = artifacts_dir.join("topology_probe_somas.csv");
    let mut somas_file = File::create(&somas_csv_path).unwrap();
    writeln!(somas_file, "soma_id,layer_index,variant_id,x,y,z").unwrap();

    let mut collision_set = HashSet::new();
    let mut collision_detected = false;

    // Keep statistics
    struct GroupStats {
        count: usize,
        z_min: u32,
        z_max: u32,
    }
    let mut summary_map: HashMap<(usize, u8), GroupStats> = HashMap::new();

    for soma in &result.somas {
        let x = soma.position.x() as u32;
        let y = soma.position.y() as u32;
        let z = soma.position.z() as u32;
        let layer_index = find_layer_index(z, &config);
        let variant_id = soma.variant_id;

        // CSV log
        writeln!(
            somas_file,
            "{},{},{},{},{},{}",
            soma.soma_id, layer_index, variant_id, x, y, z
        )
        .unwrap();

        // Collision verification
        if !collision_set.insert((x, y, z)) {
            collision_detected = true;
        }

        // Summary compilation
        let key = (layer_index, variant_id);
        let entry = summary_map.entry(key).or_insert(GroupStats {
            count: 0,
            z_min: z,
            z_max: z,
        });
        entry.count += 1;
        if z < entry.z_min {
            entry.z_min = z;
        }
        if z > entry.z_max {
            entry.z_max = z;
        }
    }

    // 5. Write summary CSV
    let summary_csv_path = artifacts_dir.join("topology_probe_summary.csv");
    let mut summary_file = File::create(&summary_csv_path).unwrap();
    writeln!(summary_file, "layer_index,variant_id,count,z_min,z_max").unwrap();

    let mut summary_keys: Vec<_> = summary_map.keys().cloned().collect();
    summary_keys.sort();

    for key in &summary_keys {
        let stats = &summary_map[key];
        writeln!(
            summary_file,
            "{},{},{},{},{}",
            key.0, key.1, stats.count, stats.z_min, stats.z_max
        )
        .unwrap();
    }

    // 5a. Write SVG visualization
    let svg_path = artifacts_dir.join("topology_probe.svg");
    let mut svg_file = File::create(&svg_path).unwrap();
    writeln!(
        svg_file,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1100 470\" width=\"100%\" height=\"100%\" style=\"background-color: \u{23}0f172a; font-family: sans-serif;\">"
    ).unwrap();
    writeln!(
        svg_file,
        "  <text x=\"20\" y=\"35\" fill=\"\u{23}f8fafc\" font-size=\"20\" font-weight=\"bold\">Single-Shard Soma Placement Visualizer (32x32x16)</text>"
    ).unwrap();
    writeln!(
        svg_file,
        "  <g transform=\"translate(700, 20)\">
    <rect x=\"0\" y=\"0\" width=\"15\" height=\"15\" fill=\"\u{23}f43f5e\" rx=\"3\"/>
    <text x=\"25\" y=\"12\" fill=\"\u{23}cbd5e1\" font-size=\"12\">Type A (variant 0)</text>
    <rect x=\"150\" y=\"0\" width=\"15\" height=\"15\" fill=\"\u{23}3b82f6\" rx=\"3\"/>
    <text x=\"175\" y=\"12\" fill=\"\u{23}cbd5e1\" font-size=\"12\">Type B (variant 1)</text>
    <rect x=\"300\" y=\"0\" width=\"15\" height=\"15\" fill=\"\u{23}10b981\" rx=\"3\"/>
    <text x=\"325\" y=\"12\" fill=\"\u{23}cbd5e1\" font-size=\"12\">Type C (variant 2)</text>
  </g>"
    )
    .unwrap();

    // XY View Group
    writeln!(
        svg_file,
        "  <g transform=\"translate(40, 80)\">
    <rect width=\"320\" height=\"320\" fill=\"\u{23}1e293b\" rx=\"5\" stroke=\"\u{23}475569\" stroke-width=\"2\"/>
    <text x=\"10\" y=\"-10\" fill=\"\u{23}94a3b8\" font-size=\"14\" font-weight=\"bold\">Top View (XY)</text>"
    ).unwrap();
    for i in 1..32 {
        writeln!(svg_file, "    <line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"320\" stroke=\"\u{23}334155\" stroke-width=\"0.5\"/>", i * 10, i * 10).unwrap();
        writeln!(svg_file, "    <line x1=\"0\" y1=\"{}\" x2=\"320\" y2=\"{}\" stroke=\"\u{23}334155\" stroke-width=\"0.5\"/>", i * 10, i * 10).unwrap();
    }
    for soma in &result.somas {
        let x = soma.position.x() as u32;
        let y = soma.position.y() as u32;
        let color = match soma.variant_id {
            0 => "\u{23}f43f5e",
            1 => "\u{23}3b82f6",
            _ => "\u{23}10b981",
        };
        writeln!(
            svg_file,
            "    <circle cx=\"{}\" cy=\"{}\" r=\"3.0\" fill=\"{}\" opacity=\"0.7\"/>",
            x * 10 + 5,
            y * 10 + 5,
            color
        )
        .unwrap();
    }
    writeln!(svg_file, "  </g>").unwrap();

    // XZ View Group
    writeln!(
        svg_file,
        "  <g transform=\"translate(400, 80)\">
    <rect width=\"320\" height=\"160\" fill=\"\u{23}1e293b\" rx=\"5\" stroke=\"\u{23}475569\" stroke-width=\"2\"/>
    <text x=\"10\" y=\"-10\" fill=\"\u{23}94a3b8\" font-size=\"14\" font-weight=\"bold\">Side View (XZ, Slice Y:12..20)</text>"
    ).unwrap();
    for i in 1..32 {
        writeln!(svg_file, "    <line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"160\" stroke=\"\u{23}334155\" stroke-width=\"0.5\"/>", i * 10, i * 10).unwrap();
    }
    for i in 1..16 {
        writeln!(svg_file, "    <line x1=\"0\" y1=\"{}\" x2=\"320\" y2=\"{}\" stroke=\"\u{23}334155\" stroke-width=\"0.5\"/>", i * 10, i * 10).unwrap();
    }
    writeln!(svg_file, "    <line x1=\"0\" y1=\"120\" x2=\"320\" y2=\"120\" stroke=\"\u{23}f43f5e\" stroke-dasharray=\"4\" stroke-width=\"1.5\"/>").unwrap();
    writeln!(svg_file, "    <line x1=\"0\" y1=\"40\" x2=\"320\" y2=\"40\" stroke=\"\u{23}f43f5e\" stroke-dasharray=\"4\" stroke-width=\"1.5\"/>").unwrap();
    for soma in &result.somas {
        let y = soma.position.y() as u32;
        if (12..=20).contains(&y) {
            let x = soma.position.x() as u32;
            let z = soma.position.z() as u32;
            let color = match soma.variant_id {
                0 => "\u{23}f43f5e",
                1 => "\u{23}3b82f6",
                _ => "\u{23}10b981",
            };
            writeln!(
                svg_file,
                "    <circle cx=\"{}\" cy=\"{}\" r=\"2.5\" fill=\"{}\" opacity=\"0.7\"/>",
                x * 10 + 5,
                160 - (z * 10 + 5),
                color
            )
            .unwrap();
        }
    }
    writeln!(svg_file, "  </g>").unwrap();

    // YZ View Group
    writeln!(
        svg_file,
        "  <g transform=\"translate(400, 280)\">
    <rect width=\"320\" height=\"160\" fill=\"\u{23}1e293b\" rx=\"5\" stroke=\"\u{23}475569\" stroke-width=\"2\"/>
    <text x=\"10\" y=\"-10\" fill=\"\u{23}94a3b8\" font-size=\"14\" font-weight=\"bold\">Side View (YZ, Slice X:12..20)</text>"
    ).unwrap();
    for i in 1..32 {
        writeln!(svg_file, "    <line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"160\" stroke=\"\u{23}334155\" stroke-width=\"0.5\"/>", i * 10, i * 10).unwrap();
    }
    for i in 1..16 {
        writeln!(svg_file, "    <line x1=\"0\" y1=\"{}\" x2=\"320\" y2=\"{}\" stroke=\"\u{23}334155\" stroke-width=\"0.5\"/>", i * 10, i * 10).unwrap();
    }
    writeln!(svg_file, "    <line x1=\"0\" y1=\"120\" x2=\"320\" y2=\"120\" stroke=\"\u{23}f43f5e\" stroke-dasharray=\"4\" stroke-width=\"1.5\"/>").unwrap();
    writeln!(svg_file, "    <line x1=\"0\" y1=\"40\" x2=\"320\" y2=\"40\" stroke=\"\u{23}f43f5e\" stroke-dasharray=\"4\" stroke-width=\"1.5\"/>").unwrap();
    for soma in &result.somas {
        let x = soma.position.x() as u32;
        if (12..=20).contains(&x) {
            let y = soma.position.y() as u32;
            let z = soma.position.z() as u32;
            let color = match soma.variant_id {
                0 => "\u{23}f43f5e",
                1 => "\u{23}3b82f6",
                _ => "\u{23}10b981",
            };
            writeln!(
                svg_file,
                "    <circle cx=\"{}\" cy=\"{}\" r=\"2.5\" fill=\"{}\" opacity=\"0.7\"/>",
                y * 10 + 5,
                160 - (z * 10 + 5),
                color
            )
            .unwrap();
        }
    }
    writeln!(svg_file, "  </g>").unwrap();

    // Summary Card Group
    writeln!(
        svg_file,
        "  <g transform=\"translate(760, 80)\">
    <rect width=\"300\" height=\"360\" fill=\"\u{23}1e293b\" rx=\"5\" stroke=\"\u{23}475569\" stroke-width=\"2\"/>
    <text x=\"20\" y=\"35\" fill=\"\u{23}f8fafc\" font-size=\"16\" font-weight=\"bold\">Placements Stats</text>
    <text x=\"20\" y=\"70\" fill=\"\u{23}94a3b8\" font-size=\"13\">Somas Placed:</text>
    <text x=\"150\" y=\"70\" fill=\"\u{23}f8fafc\" font-size=\"13\" font-weight=\"bold\">{}</text>
    <text x=\"20\" y=\"95\" fill=\"\u{23}94a3b8\" font-size=\"13\">Shard Capacity:</text>
    <text x=\"150\" y=\"95\" fill=\"\u{23}f8fafc\" font-size=\"13\" font-weight=\"bold\">{} voxels</text>
    <text x=\"20\" y=\"120\" fill=\"\u{23}94a3b8\" font-size=\"13\">Occupancy Ratio:</text>
    <text x=\"150\" y=\"120\" fill=\"\u{23}f8fafc\" font-size=\"13\" font-weight=\"bold\">{:.4}%</text>
    <text x=\"20\" y=\"145\" fill=\"\u{23}94a3b8\" font-size=\"13\">Collision Check:</text>
    <text x=\"150\" y=\"145\" fill=\"{}\" font-size=\"13\" font-weight=\"bold\">{}</text>
    
    <text x=\"20\" y=\"185\" fill=\"\u{23}f8fafc\" font-size=\"14\" font-weight=\"bold\">Counts by Layer/Variant</text>
    <line x1=\"20\" y1=\"195\" x2=\"280\" y2=\"195\" stroke=\"\u{23}475569\" stroke-width=\"1\"/>",
        total_somas,
        shard_capacity,
        occupancy_ratio * 100.0,
        if collision_detected { "\u{23}f43f5e" } else { "\u{23}10b981" },
        if collision_detected { "FAILED" } else { "PASSED" }
    ).unwrap();

    let mut y_offset = 220;
    for key in &summary_keys {
        let stats = &summary_map[key];
        let variant_name = match key.1 {
            0 => "Type A (v0)",
            1 => "Type B (v1)",
            _ => "Type C (v2)",
        };
        writeln!(
            svg_file,
            "    <text x=\"20\" y=\"{}\" fill=\"\u{23}94a3b8\" font-size=\"12\">L{} - {}:</text>
    <text x=\"160\" y=\"{}\" fill=\"\u{23}f8fafc\" font-size=\"12\" font-weight=\"bold\">{}</text>
    <text x=\"200\" y=\"{}\" fill=\"\u{23}64748b\" font-size=\"11\">[Z: {}..={}]</text>",
            y_offset,
            key.0,
            variant_name,
            y_offset,
            stats.count,
            y_offset,
            stats.z_min,
            stats.z_max
        )
        .unwrap();
        y_offset += 25;
    }

    writeln!(svg_file, "  </g>\n</svg>").unwrap();

    // 6. Print console summary
    println!("1. Somas Placed: {}", total_somas);
    println!("2. Shard Capacity: {} voxels", shard_capacity);
    println!("3. Occupancy Ratio: {:.4}%", occupancy_ratio * 100.0);
    println!(
        "4. Somas Collision Check: {}",
        if collision_detected {
            "FAILED (Collisions found)"
        } else {
            "PASSED (0 Collisions)"
        }
    );
    println!("5. Placements Summary per Layer/Variant:");
    println!("   Layer | Variant | Count | Z-Bounds");
    println!("   ----------------------------------");
    for key in &summary_keys {
        let stats = &summary_map[key];
        println!(
            "   {:>5} | {:>7} | {:>5} | [{}..={}]",
            key.0, key.1, stats.count, stats.z_min, stats.z_max
        );
    }

    println!("\nCSV artifacts generated successfully:");
    println!("  - Somas log: {:?}", somas_csv_path);
    println!("  - Summary log: {:?}", summary_csv_path);
}
