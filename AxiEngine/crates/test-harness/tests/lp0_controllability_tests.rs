//! LP-0: Frozen / Plasticity Controllability Integration Tests

#![cfg(feature = "full-chain-probe")]

use std::fs::{remove_file, File};
use std::io::Write;
use std::path::PathBuf;

use baker::{bake_local_shard, pack_local_shard_artifacts, LocalShardBakeInput};
use boot::{bootstrap_local_shard_engine, LocalRuntimeBootExt, LocalShardComputeInput};
use runtime::{LocalRuntime, LocalRuntimeConfig};
use test_harness::compute_dendrite_weights_checksum;
use types::MasterSeed;

fn create_test_engine_and_bundle() -> (compute::ShardEngine, boot::LocalShardBootBundle, PathBuf) {
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
            // Enable massive plasticity to ensure weights change when enabled
            gsop_potentiation: 100,
            gsop_depression: 100,
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
            // Spontaneous firing to guarantee spikes and STDP activity
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
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let packed = pack_local_shard_artifacts(&artifacts).unwrap();

    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("lp0_vertical_slice_{}.axic", rand));
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

#[test]
fn test_frozen_plasticity_controllability_lp0() {
    let (engine1, bundle1, path1) = create_test_engine_and_bundle();
    let (engine2, bundle2, path2) = create_test_engine_and_bundle();

    let padded_n = bundle1.spec.padded_n;

    // 1. Run under plasticity_enabled = false
    let config_frozen = LocalRuntimeConfig {
        sync_batch_ticks: 20,
        v_seg: 1,
        dopamine: 50,
        max_spikes_per_tick: 10,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_words_per_tick: 1,
        mapped_soma_ids: vec![0, 1, 2, 3],
        plasticity_enabled: false,
    };

    let mut runtime_frozen =
        LocalRuntime::from_boot_bundle(engine1, config_frozen, bundle1).unwrap();

    // Write a mock active synapse target to soma 0 slot 0 to allow plasticity rule updates
    {
        runtime_frozen.engine_mut().enter_maintenance().unwrap();
        let mut working = runtime_frozen.working_state_mut().unwrap().clone();
        let maintenance_mut = compute_api::BackendMaintenanceMut {
            state_blob: &mut working.state_blob,
            axons_blob: &mut working.axons_blob,
        };
        runtime_frozen
            .engine_mut()
            .export_maintenance_state(maintenance_mut)
            .unwrap();

        let offsets = layout::offsets::compute_state_offsets(padded_n as usize);

        // Write active targets to all 4 somas in slot 0, pointing to axons 0..3 respectively
        for i in 0..4u32 {
            let i_usize = i as usize;
            let target = types::PackedTarget::pack(i, 0);
            let t_bytes = target.0.to_le_bytes();
            working.state_blob
                [offsets.off_targets + i_usize * 4..offsets.off_targets + i_usize * 4 + 4]
                .copy_from_slice(&t_bytes);

            // Initial weight = 50000 (middle of the range to allow updates in both directions)
            let w_bytes = 50000i32.to_le_bytes();
            working.state_blob
                [offsets.off_weights + i_usize * 4..offsets.off_weights + i_usize * 4 + 4]
                .copy_from_slice(&w_bytes);
        }

        let maintenance_ref = compute_api::BackendMaintenanceRef {
            state_blob: &working.state_blob,
            axons_blob: &working.axons_blob,
        };
        runtime_frozen
            .engine_mut()
            .import_maintenance_state(maintenance_ref)
            .unwrap();
        runtime_frozen.engine_mut().exit_maintenance().unwrap();
        *runtime_frozen.working_state_mut().unwrap() = working;
    }

    // Create active input spikes on all axons [0, 1, 2, 3] to trigger STDP
    let sync_ticks = 20;
    let max_spikes = 10;
    let mut incoming_spikes = vec![0u32; sync_ticks * max_spikes];
    let mut incoming_spike_counts = vec![0u32; sync_ticks];
    for t in 0..sync_ticks {
        incoming_spikes[t * max_spikes] = 0;
        incoming_spikes[t * max_spikes + 1] = 1;
        incoming_spikes[t * max_spikes + 2] = 2;
        incoming_spikes[t * max_spikes + 3] = 3;
        incoming_spike_counts[t] = 4;
    }
    let input = runtime::RuntimeBatchInput {
        input_bitmask: None,
        incoming_spikes: Some(&incoming_spikes),
        incoming_spike_counts: &incoming_spike_counts,
    };

    let state_before_frozen = runtime_frozen.working_state().unwrap().state_blob.clone();
    let checksum_before_frozen =
        compute_dendrite_weights_checksum(&state_before_frozen, padded_n as usize);

    // Day batch execution
    let _report_frozen = runtime_frozen.run_batch(input).unwrap();
    sync_state_from_engine(&mut runtime_frozen);

    let state_after_frozen = runtime_frozen.working_state().unwrap().state_blob.clone();
    let checksum_after_frozen =
        compute_dendrite_weights_checksum(&state_after_frozen, padded_n as usize);

    // Invariant Check 1: Weights must NOT change when plasticity is disabled
    assert_eq!(
        checksum_before_frozen, checksum_after_frozen,
        "LP-0 Gate: Weights changed when plasticity_enabled = false"
    );

    // 2. Run under plasticity_enabled = true
    let config_plastic = LocalRuntimeConfig {
        sync_batch_ticks: 20,
        v_seg: 1,
        dopamine: 50,
        max_spikes_per_tick: 10,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_words_per_tick: 1,
        mapped_soma_ids: vec![0, 1, 2, 3],
        plasticity_enabled: true,
    };

    let mut runtime_plastic =
        LocalRuntime::from_boot_bundle(engine2, config_plastic, bundle2).unwrap();

    // Write the exact same mock active synapse targets
    {
        runtime_plastic.engine_mut().enter_maintenance().unwrap();
        let mut working = runtime_plastic.working_state_mut().unwrap().clone();
        let maintenance_mut = compute_api::BackendMaintenanceMut {
            state_blob: &mut working.state_blob,
            axons_blob: &mut working.axons_blob,
        };
        runtime_plastic
            .engine_mut()
            .export_maintenance_state(maintenance_mut)
            .unwrap();

        let offsets = layout::offsets::compute_state_offsets(padded_n as usize);
        for i in 0..4u32 {
            let i_usize = i as usize;
            let target = types::PackedTarget::pack(i, 0);
            let t_bytes = target.0.to_le_bytes();
            working.state_blob
                [offsets.off_targets + i_usize * 4..offsets.off_targets + i_usize * 4 + 4]
                .copy_from_slice(&t_bytes);

            let w_bytes = 50000i32.to_le_bytes();
            working.state_blob
                [offsets.off_weights + i_usize * 4..offsets.off_weights + i_usize * 4 + 4]
                .copy_from_slice(&w_bytes);
        }

        let maintenance_ref = compute_api::BackendMaintenanceRef {
            state_blob: &working.state_blob,
            axons_blob: &working.axons_blob,
        };
        runtime_plastic
            .engine_mut()
            .import_maintenance_state(maintenance_ref)
            .unwrap();
        runtime_plastic.engine_mut().exit_maintenance().unwrap();
        *runtime_plastic.working_state_mut().unwrap() = working;
    }

    let state_before_plastic = runtime_plastic.working_state().unwrap().state_blob.clone();
    let checksum_before_plastic =
        compute_dendrite_weights_checksum(&state_before_plastic, padded_n as usize);

    // Ensure we start with matching checksums for both runs
    assert_eq!(checksum_before_frozen, checksum_before_plastic);

    // Day batch execution
    let _report_plastic = runtime_plastic.run_batch(input).unwrap();
    sync_state_from_engine(&mut runtime_plastic);

    let state_after_plastic = runtime_plastic.working_state().unwrap().state_blob.clone();
    let checksum_after_plastic =
        compute_dendrite_weights_checksum(&state_after_plastic, padded_n as usize);

    // Invariant Check 2: Weights MUST change under stimulus when plasticity is enabled
    assert_ne!(
        checksum_before_plastic, checksum_after_plastic,
        "LP-0 Gate: Weights did not change when plasticity_enabled = true"
    );

    // Invariant Check 3: Electrical dynamics must be IDENTICAL between plastic and frozen runs
    // Check soma potentials (excluding dendrite_weights and timers, which only record mass/GSOP)
    let offsets = layout::offsets::compute_state_offsets(padded_n as usize);
    let voltage_range = offsets.off_voltage..offsets.off_flags;

    assert_eq!(
        state_after_frozen[voltage_range.clone()],
        state_after_plastic[voltage_range],
        "LP-0 Gate: Flipped plasticity_enabled flag unexpectedly altered electrical membrane potentials"
    );

    let _ = remove_file(path1);
    let _ = remove_file(path2);
}
