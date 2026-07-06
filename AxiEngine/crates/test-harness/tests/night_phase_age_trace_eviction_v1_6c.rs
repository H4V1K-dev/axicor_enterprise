#[derive(Debug, Clone, serde::Serialize)]
struct DormantSynapse {
    source_soma_id: u32,
    target_soma_id: u32,
    flat_segment_idx: u32,
    weight: i32,
    long_trace: u16,
    short_trace: u16,
    dormant_age: u32,
    projection_class: String,
    pre_trace_timer: u8,
    initial_weight: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CycleMetricsV16c {
    cycle: usize,
    dormant_count: usize,
    dead_count: usize,
    age_trace_evicted: usize,
    target_cap_evicted: usize,
    global_cap_evicted: usize,
    dormant_age_p50: u32,
    dormant_age_p90: u32,
    dormant_age_max: u32,
    dormant_long_trace_p50: u16,
    dormant_long_trace_p90: u16,
    dormant_long_trace_max: u16,
    max_dormant_per_target: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct PlottingDataV16c {
    cycles: Vec<CycleMetricsV16c>,
}

#[test]
fn run_night_phase_age_trace_eviction_v1_6c() {
    println!("=== Starting Night Phase MVP Age+Trace Eviction Probe v1.6c ===");

    let mut dormant_synapses = Vec::new();
    let n_dormant = 100;

    // Populate synthetic dormant bank
    // Assign each entry a unique target_soma_id so target caps (max 10) are never hit!
    for i in 0..n_dormant {
        dormant_synapses.push(DormantSynapse {
            source_soma_id: i as u32,
            target_soma_id: i as u32,
            flat_segment_idx: 0,
            weight: 1500 << 16,
            long_trace: 0,
            short_trace: 0,
            dormant_age: 0,
            projection_class: "L4->L4".to_string(),
            pre_trace_timer: 0,
            initial_weight: 1500 << 16,
        });
    }

    let mut dead_count = 0;
    let max_dormant_age = 2;
    let max_dormant_total = 500;
    let max_dormant_per_target = 10;
    let total_cycles = 4;

    let mut cycle_metrics_list = Vec::new();

    for cycle in 1..=total_cycles {
        println!("  Cycle {} / {}", cycle, total_cycles);

        // 1. Trace decay / age increment happens before eviction
        for ds in dormant_synapses.iter_mut() {
            ds.short_trace = ds.short_trace.saturating_sub(ds.short_trace >> 1);
            ds.long_trace = ds.long_trace.saturating_sub(ds.long_trace >> 4);
            ds.dormant_age += 1;
        }

        // 2. Apply age+trace eviction
        let mut fresh_dormant = Vec::new();
        let mut age_trace_evicted = 0;

        for ds in dormant_synapses {
            if ds.dormant_age > max_dormant_age && ds.long_trace == 0 {
                dead_count += 1;
                age_trace_evicted += 1;
            } else {
                fresh_dormant.push(ds);
            }
        }
        dormant_synapses = fresh_dormant;

        // 3. Apply target cap eviction
        let mut target_cap_evicted = 0;
        let mut by_target: std::collections::HashMap<u32, Vec<DormantSynapse>> =
            std::collections::HashMap::new();
        for ds in dormant_synapses {
            by_target.entry(ds.target_soma_id).or_default().push(ds);
        }
        let mut final_dormant = Vec::new();
        for (_, target_list) in by_target {
            if target_list.len() > max_dormant_per_target {
                let evict_count = target_list.len() - max_dormant_per_target;
                target_cap_evicted += evict_count;
                dead_count += evict_count;
                final_dormant.extend(target_list.into_iter().skip(evict_count));
            } else {
                final_dormant.extend(target_list);
            }
        }
        dormant_synapses = final_dormant;

        // 4. Apply global cap eviction
        let mut global_cap_evicted = 0;
        if dormant_synapses.len() > max_dormant_total {
            let evict_count = dormant_synapses.len() - max_dormant_total;
            global_cap_evicted += evict_count;
            dead_count += evict_count;
            dormant_synapses = dormant_synapses.into_iter().skip(evict_count).collect();
        }

        // Compute metrics
        let mut ages: Vec<u32> = dormant_synapses.iter().map(|d| d.dormant_age).collect();
        ages.sort();
        let dormant_age_p50 = if ages.is_empty() {
            0
        } else {
            ages[ages.len() / 2]
        };
        let dormant_age_p90 = if ages.is_empty() {
            0
        } else {
            ages[(ages.len() as f64 * 0.9) as usize % ages.len()]
        };
        let dormant_age_max = ages.iter().max().cloned().unwrap_or(0);

        let mut long_traces: Vec<u16> = dormant_synapses.iter().map(|d| d.long_trace).collect();
        long_traces.sort();
        let dormant_long_trace_p50 = if long_traces.is_empty() {
            0
        } else {
            long_traces[long_traces.len() / 2]
        };
        let dormant_long_trace_p90 = if long_traces.is_empty() {
            0
        } else {
            long_traces[(long_traces.len() as f64 * 0.9) as usize % long_traces.len()]
        };
        let dormant_long_trace_max = long_traces.iter().max().cloned().unwrap_or(0);

        let mut dormant_counts_per_target = std::collections::HashMap::new();
        for ds in &dormant_synapses {
            *dormant_counts_per_target
                .entry(ds.target_soma_id)
                .or_insert(0) += 1;
        }
        let max_dormant_per_target = dormant_counts_per_target
            .values()
            .max()
            .cloned()
            .unwrap_or(0);

        cycle_metrics_list.push(CycleMetricsV16c {
            cycle,
            dormant_count: dormant_synapses.len(),
            dead_count,
            age_trace_evicted,
            target_cap_evicted,
            global_cap_evicted,
            dormant_age_p50,
            dormant_age_p90,
            dormant_age_max,
            dormant_long_trace_p50,
            dormant_long_trace_p90,
            dormant_long_trace_max,
            max_dormant_per_target,
        });
    }

    // Assert final results according to specs
    assert_eq!(dormant_synapses.len(), 0);
    assert_eq!(dead_count, n_dormant);

    // Save plotting data
    let archive_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/engine/research/archive/2026-07-06_night_phase_age_trace_eviction_v1_6c/artifacts");
    std::fs::create_dir_all(&archive_dir).expect("Failed to create archive artifacts dir");
    let json_path = archive_dir.join("plot_data.json");
    let data = PlottingDataV16c {
        cycles: cycle_metrics_list,
    };
    let json_str = serde_json::to_string_pretty(&data).unwrap();
    std::fs::write(&json_path, json_str).expect("Failed to write plot_data.json");
    println!("Saved v1.6c plotting data to {:?}", json_path);
}
