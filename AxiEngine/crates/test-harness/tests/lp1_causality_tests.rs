//! LP-1: Plasticity Causality (Correlated vs Control) Integration Tests
//!
//! Verifies that production GSOP/STDP rule selectively strengthens correlated pathways
//! compared to unmatched control pathways under deterministic spiking stimulus.

#![cfg(feature = "full-chain-probe")]

use std::fs::{remove_file, File};
use std::io::Write;
use std::path::PathBuf;

use baker::{bake_local_shard, pack_local_shard_artifacts, LocalShardBakeInput};
use boot::{bootstrap_local_shard_engine, LocalRuntimeBootExt, LocalShardComputeInput};
use runtime::{LocalRuntime, LocalRuntimeConfig};
use types::MasterSeed;

fn create_test_engine_and_bundle(
    seed: u64,
) -> (compute::ShardEngine, boot::LocalShardBootBundle, PathBuf) {
    let neuron_types = vec![config::NeuronType {
        name: "TypeA".to_string(),
        membrane: config::MembraneParams {
            threshold: 1000,
            rest_potential: -70,
            leak_shift: 1,
            ahp_amplitude: 5,
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
            d1_affinity: 100,
            d2_affinity: 100,
        },
        gsop: config::GsopParams {
            gsop_potentiation: 100,
            gsop_depression: 10,
            initial_synapse_weight: 10000,
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
            // Spontaneous firing to guarantee postsynaptic spikes for correlation evaluation
            spontaneous_firing_period_ticks: 5,
        },
    }];
    let layers = vec![config::LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.2,
        composition: vec![config::NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let shard_config = config::ShardConfig {
        meta: None,
        dimensions: config::ShardDimensions {
            w: 20,
            d: 20,
            h: 20,
        },
        settings: config::ShardSettings {
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
    let input = LocalShardBakeInput {
        shard_config: &shard_config,
        master_seed: MasterSeed(seed),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let packed = pack_local_shard_artifacts(&artifacts).unwrap();

    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("lp1_vertical_slice_{}_{}.axic", seed, rand));
    {
        let mut f = File::create(&temp).unwrap();
        f.write_all(&packed).unwrap();
    }

    let compute_input = LocalShardComputeInput {
        archive_path: temp.clone(),
        backend_preference: compute::BackendPreference::Cpu,
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let (engine, bundle) = bootstrap_local_shard_engine(&compute_input).unwrap();
    (engine, bundle, temp)
}

fn sync_state_from_engine(runtime: &mut LocalRuntime) {
    runtime.engine_mut().enter_maintenance().unwrap();
    let mut working = runtime.working_state().unwrap().clone();
    let maintenance_mut = compute_api::BackendMaintenanceMut {
        state_blob: &mut working.state_blob,
        axons_blob: &mut working.axons_blob,
    };
    runtime
        .engine_mut()
        .export_maintenance_state(maintenance_mut)
        .unwrap();
    runtime.engine_mut().exit_maintenance().unwrap();
    *runtime.working_state_mut().unwrap() = working;
}

fn write_target(
    state_blob: &mut [u8],
    padded_n: usize,
    soma_idx: usize,
    slot_idx: usize,
    target: types::PackedTarget,
) {
    let offsets = layout::offsets::compute_state_offsets(padded_n);
    let idx = slot_idx * padded_n + soma_idx;
    let bytes = target.0.to_le_bytes();
    state_blob[offsets.off_targets + idx * 4..offsets.off_targets + idx * 4 + 4]
        .copy_from_slice(&bytes);
}

fn write_weight(
    state_blob: &mut [u8],
    padded_n: usize,
    soma_idx: usize,
    slot_idx: usize,
    weight: i32,
) {
    let offsets = layout::offsets::compute_state_offsets(padded_n);
    let idx = slot_idx * padded_n + soma_idx;
    let bytes = weight.to_le_bytes();
    state_blob[offsets.off_weights + idx * 4..offsets.off_weights + idx * 4 + 4]
        .copy_from_slice(&bytes);
}

fn read_weight(state_blob: &[u8], padded_n: usize, soma_idx: usize, slot_idx: usize) -> i32 {
    let offsets = layout::offsets::compute_state_offsets(padded_n);
    let idx = slot_idx * padded_n + soma_idx;
    i32::from_le_bytes(
        state_blob[offsets.off_weights + idx * 4..offsets.off_weights + idx * 4 + 4]
            .try_into()
            .unwrap(),
    )
}

#[test]
fn test_causality_correlated_vs_control_lp1() {
    let seeds = [42, 100, 2026];

    for &seed in &seeds {
        println!("------------------------------------------------------------");
        println!(
            "Running LP-1 Plasticity Causality Experiment for Seed: {}",
            seed
        );
        let (engine, bundle, path) = create_test_engine_and_bundle(seed);
        let padded_n = bundle.spec.padded_n;

        let config = LocalRuntimeConfig {
            sync_batch_ticks: 20,
            v_seg: 1,
            dopamine: 50,
            max_spikes_per_tick: 10,
            virtual_offset: 0,
            num_virtual_axons: 0,
            input_words_per_tick: 1,
            mapped_soma_ids: vec![0],
            plasticity_enabled: true,
        };

        let mut runtime = LocalRuntime::from_boot_bundle(engine, config, bundle).unwrap();

        // Configure mock active targets & weights inside working state blob
        let initial_weight = 100i32 << 16; // 6553600 (non-zero charge weight)
        {
            runtime.engine_mut().enter_maintenance().unwrap();
            let mut working = runtime.working_state_mut().unwrap().clone();
            let maintenance_mut = compute_api::BackendMaintenanceMut {
                state_blob: &mut working.state_blob,
                axons_blob: &mut working.axons_blob,
            };
            runtime
                .engine_mut()
                .export_maintenance_state(maintenance_mut)
                .unwrap();

            // soma 0, slot 0 -> axon 0 (correlated)
            write_target(
                &mut working.state_blob,
                padded_n as usize,
                0,
                0,
                types::PackedTarget::pack(0, 0),
            );
            write_weight(
                &mut working.state_blob,
                padded_n as usize,
                0,
                0,
                initial_weight,
            );

            // soma 0, slot 1 -> axon 1 (control)
            write_target(
                &mut working.state_blob,
                padded_n as usize,
                0,
                1,
                types::PackedTarget::pack(1, 0),
            );
            write_weight(
                &mut working.state_blob,
                padded_n as usize,
                0,
                1,
                initial_weight,
            );

            let maintenance_ref = compute_api::BackendMaintenanceRef {
                state_blob: &working.state_blob,
                axons_blob: &working.axons_blob,
            };
            runtime
                .engine_mut()
                .import_maintenance_state(maintenance_ref)
                .unwrap();
            runtime.engine_mut().exit_maintenance().unwrap();
            *runtime.working_state_mut().unwrap() = working;
        }

        // Run day simulation batches (total 100 ticks = 5 batches * 20 ticks)
        let sync_ticks = 20;
        let max_spikes = 10;
        let mut incoming_spikes = vec![0u32; sync_ticks * max_spikes];
        let mut incoming_spike_counts = vec![0u32; sync_ticks];
        for t in 0..sync_ticks {
            // Axon 0 is co-active/correlated (spikes every tick)
            incoming_spikes[t * max_spikes] = 0;
            incoming_spike_counts[t] = 1;
        }

        for _ in 0..5 {
            let input = runtime::RuntimeBatchInput {
                input_bitmask: None,
                incoming_spikes: Some(&incoming_spikes),
                incoming_spike_counts: &incoming_spike_counts,
            };
            runtime.run_batch(input).unwrap();
        }

        // Export state blob to check updated weights
        sync_state_from_engine(&mut runtime);
        let final_state_blob = &runtime.working_state().unwrap().state_blob;

        let w_correlated = read_weight(final_state_blob, padded_n as usize, 0, 0);
        let w_control = read_weight(final_state_blob, padded_n as usize, 0, 1);

        let delta_correlated = w_correlated - initial_weight;
        let delta_control = w_control - initial_weight;

        println!("Seed {}:", seed);
        println!(
            "  Initial Weight: {} (charge = {})",
            initial_weight,
            initial_weight >> 16
        );
        println!(
            "  Correlated final weight: {} (charge = {}) [delta = {}]",
            w_correlated,
            w_correlated >> 16,
            delta_correlated
        );
        println!(
            "  Control final weight: {} (charge = {}) [delta = {}]",
            w_control,
            w_control >> 16,
            delta_control
        );

        // Success Gates checks
        assert!(
            delta_correlated > delta_control,
            "Causality Violation on Seed {}: delta_correlated ({}) must be strictly greater than delta_control ({})",
            seed, delta_correlated, delta_control
        );

        // Dale's law verification (weights sign must not flip)
        assert!(
            w_correlated > 0,
            "Dale Violation: w_correlated became non-positive"
        );
        assert!(
            w_control > 0,
            "Dale Violation: w_control became non-positive"
        );

        // Bounds verification
        assert!(
            w_correlated <= physics::constants::MAX_WEIGHT_LIMIT,
            "Bound Violation: w_correlated exceeded MAX_WEIGHT_LIMIT"
        );
        assert!(
            w_control <= physics::constants::MAX_WEIGHT_LIMIT,
            "Bound Violation: w_control exceeded MAX_WEIGHT_LIMIT"
        );

        let _ = remove_file(path);
    }
}
