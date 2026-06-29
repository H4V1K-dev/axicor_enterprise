//! Probe diagnostic execution script for `compute-cpu`.
//!
//! NOTE: This probe is purely diagnostic for local integration verification and performance probing;
//! it is NOT an architectural Single Source of Truth (SSOT) or specification contract.

use compute_api::*;
use compute_cpu::*;
use layout::{compute_state_offsets, VariantParameters, VARIANT_LUT_LEN};
use std::fs::{create_dir_all, File};
use std::io::Write;
use types::PackedTarget;

fn main() {
    println!("=== AxiEngine compute-cpu Diagnostic Probe ===");

    let padded_n = 64;
    let total_axons = 10;
    let sync_ticks = 20;

    let mut backend = CpuBackend::new(CpuBackendConfig::default()).unwrap();
    let spec = ShardAllocSpec {
        padded_n: padded_n as u32,
        total_axons: total_axons as u32,
        total_ghosts: 0,
        virtual_offset: 0,
    };

    let handle = backend.alloc_shard(spec).expect("Failed to allocate shard");

    // Prepare state blob and axons blob
    let offsets = compute_state_offsets(padded_n);
    let mut state_blob = vec![0u8; offsets.total_state_size];
    let axons_blob = vec![0u8; validation::expected_axons_blob_size(total_axons as u32).unwrap()];

    // Configure Soma 0 to connect to Axon 0
    let (s2a_bytes, rest) = state_blob.split_at_mut(offsets.off_targets);
    let soma_to_axon = bytemuck::cast_slice_mut::<u8, u32>(&mut s2a_bytes[offsets.off_s2a..]);
    soma_to_axon[0] = 0;

    // Configure Soma 1 to connect dendrite 0 to Axon 0, segment 0
    let (targets_bytes, rest2) = rest.split_at_mut(offsets.off_weights - offsets.off_targets);
    let dendrite_targets = bytemuck::cast_slice_mut::<u8, u32>(
        &mut targets_bytes[..layout::MAX_DENDRITES * padded_n * 4],
    );
    dendrite_targets[1] = PackedTarget::pack(0, 0).0;

    let weights_bytes = &mut rest2[..layout::MAX_DENDRITES * padded_n * 4];
    let dendrite_weights = bytemuck::cast_slice_mut::<u8, i32>(weights_bytes);
    dendrite_weights[1] = 6553600; // Initial mass (~100 charge)

    // Variant table: profile 0 with low threshold to respond quickly
    let mut variant_table = [VariantParameters {
        threshold: 50,
        rest_potential: 0,
        leak_shift: 10,
        homeostasis_penalty: 10,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 1000,
        gsop_potentiation: 100,
        gsop_depression: 50,
        homeostasis_decay: 1,
        refractory_period: 2,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [128; 8],
        ahp_amplitude: 10,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    }; VARIANT_LUT_LEN];

    // Profile 1 for heartbeat
    variant_table[1].heartbeat_m = 65535;

    backend
        .upload_shard(
            handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variant_table,
            },
        )
        .expect("Upload failed");

    // Input bitmask: inject virtual input to axon 0 on tick 0
    let mut bitmask = vec![0u32; sync_ticks];
    bitmask[0] = 1; // Virtual axon 0 fires on tick 0

    let mut output_spikes = vec![0u32; sync_ticks * 10];
    let mut output_counts = vec![0u32; sync_ticks];
    let incoming_counts = vec![0u32; sync_ticks];
    let mapped_somas = vec![0u32, 1u32];

    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: sync_ticks as u32,
        v_seg: 1,
        dopamine: 10,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 2,
        virtual_offset: 0,
        num_virtual_axons: 1,
        input_bitmask: Some(&bitmask),
        incoming_spikes: None,
        incoming_spike_counts: &incoming_counts,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_counts,
    };

    let result = backend
        .run_day_batch(handle, cmd)
        .expect("Batch execution failed");
    println!("Execution completed in {} us", result.execution_time_us);
    println!("Generated spikes: {}", result.generated_spikes_count);
    println!("Output spikes written: {}", result.output_spikes_written);

    // Extract snapshot to inspect final state & weights
    let mut snap_state = vec![0u8; state_blob.len()];
    let mut snap_axons = vec![0u8; axons_blob.len()];
    backend
        .debug_snapshot(
            handle,
            ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .expect("Snapshot failed");

    let _ = create_dir_all("artifacts");

    // Save ticks CSV
    let mut ticks_file =
        File::create("artifacts/compute_cpu_probe_ticks.csv").expect("Create ticks.csv failed");
    writeln!(ticks_file, "tick,output_spike_count").unwrap();
    for (tick, count) in output_counts.iter().enumerate().take(sync_ticks) {
        writeln!(ticks_file, "{},{}", tick, count).unwrap();
    }

    // Save weights CSV
    let snap_offsets = compute_state_offsets(padded_n);
    let snap_weights_bytes = &snap_state[snap_offsets.off_weights..snap_offsets.off_dtimers];
    let snap_weights = bytemuck::cast_slice::<u8, i32>(snap_weights_bytes);

    let mut weights_file =
        File::create("artifacts/compute_cpu_probe_weights.csv").expect("Create weights.csv failed");
    writeln!(weights_file, "soma_id,dendrite_id,weight").unwrap();
    writeln!(weights_file, "1,0,{}", snap_weights[1]).unwrap();

    println!("Diagnostic artifacts written to artifacts/ directory successfully.");
}
