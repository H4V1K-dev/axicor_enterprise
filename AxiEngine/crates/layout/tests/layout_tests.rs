use core::mem::{align_of, offset_of, size_of};
use layout::*;
use static_assertions::const_assert_eq;

#[test]
fn test_abi_sizes_and_alignments() {
    const_assert_eq!(size_of::<VariantParameters>(), 64);
    const_assert_eq!(align_of::<VariantParameters>(), 64);

    const_assert_eq!(size_of::<BurstHeads8>(), 32);
    const_assert_eq!(align_of::<BurstHeads8>(), 32);

    const_assert_eq!(size_of::<StateFileHeader>(), 16);
    const_assert_eq!(align_of::<StateFileHeader>(), 16);

    const_assert_eq!(size_of::<AxonsFileHeader>(), 16);
    const_assert_eq!(align_of::<AxonsFileHeader>(), 16);

    const_assert_eq!(size_of::<PathsFileHeader>(), 16);
    const_assert_eq!(align_of::<PathsFileHeader>(), 16);

    const_assert_eq!(size_of::<types::PackedTarget>(), 4);
}

#[test]
fn test_variant_parameters_field_offsets() {
    assert_eq!(offset_of!(VariantParameters, threshold), 0);
    assert_eq!(offset_of!(VariantParameters, rest_potential), 4);
    assert_eq!(offset_of!(VariantParameters, leak_shift), 8);
    assert_eq!(offset_of!(VariantParameters, homeostasis_penalty), 12);
    assert_eq!(
        offset_of!(VariantParameters, spontaneous_firing_period_ticks),
        16
    );
    assert_eq!(offset_of!(VariantParameters, initial_synapse_weight), 20);
    assert_eq!(offset_of!(VariantParameters, gsop_potentiation), 22);
    assert_eq!(offset_of!(VariantParameters, gsop_depression), 24);
    assert_eq!(offset_of!(VariantParameters, homeostasis_decay), 26);
    assert_eq!(offset_of!(VariantParameters, refractory_period), 28);
    assert_eq!(offset_of!(VariantParameters, synapse_refractory_period), 29);
    assert_eq!(offset_of!(VariantParameters, signal_propagation_length), 30);
    assert_eq!(offset_of!(VariantParameters, is_inhibitory), 31);
    assert_eq!(offset_of!(VariantParameters, inertia_curve), 32);
    assert_eq!(offset_of!(VariantParameters, ahp_amplitude), 40);
    assert_eq!(offset_of!(VariantParameters, _pad1), 42);
    assert_eq!(offset_of!(VariantParameters, adaptive_leak_min_shift), 48);
    assert_eq!(offset_of!(VariantParameters, adaptive_leak_gain), 52);
    assert_eq!(offset_of!(VariantParameters, adaptive_mode), 54);
    assert_eq!(offset_of!(VariantParameters, _leak_pad), 55);
    assert_eq!(offset_of!(VariantParameters, d1_affinity), 58);
    assert_eq!(offset_of!(VariantParameters, d2_affinity), 59);
    assert_eq!(offset_of!(VariantParameters, heartbeat_m), 60);
}

#[test]
fn test_burst_heads_sentinel_init() {
    let burst = BurstHeads8::empty(types::AXON_SENTINEL);
    assert_eq!(burst.h0, types::AXON_SENTINEL);
    assert_eq!(burst.h1, types::AXON_SENTINEL);
    assert_eq!(burst.h2, types::AXON_SENTINEL);
    assert_eq!(burst.h3, types::AXON_SENTINEL);
    assert_eq!(burst.h4, types::AXON_SENTINEL);
    assert_eq!(burst.h5, types::AXON_SENTINEL);
    assert_eq!(burst.h6, types::AXON_SENTINEL);
    assert_eq!(burst.h7, types::AXON_SENTINEL);
}

#[test]
fn test_align_matrix() {
    assert_eq!(align64(0), 0);
    assert_eq!(align64(1), 64);
    assert_eq!(align64(32), 64);
    assert_eq!(align64(33), 64);
    assert_eq!(align64(63), 64);
    assert_eq!(align64(64), 64);
    assert_eq!(align64(65), 128);

    assert_eq!(align_to_padded_n(33), 64);
    assert_eq!(align_to_warp(33), 64);
}

#[test]
fn test_state_offsets_calculation() {
    let offsets = compute_state_offsets(64);
    assert_eq!(offsets.off_voltage, 64);
    assert_eq!(offsets.off_flags, 320);
    assert_eq!(offsets.off_thresh, 384);
    assert_eq!(offsets.off_timers, 640);
    assert_eq!(offsets.off_s2a, 704);
    assert_eq!(offsets.off_targets, 960);
    assert_eq!(offsets.off_weights, 33728);
    assert_eq!(offsets.off_dtimers, 66496);
    assert_eq!(offsets.total_state_size, 74688);

    assert_eq!(offsets.off_voltage % 64, 0);
    assert_eq!(offsets.off_flags % 64, 0);
    assert_eq!(offsets.off_thresh % 64, 0);
    assert_eq!(offsets.off_timers % 64, 0);
    assert_eq!(offsets.off_s2a % 64, 0);
    assert_eq!(offsets.off_targets % 64, 0);
    assert_eq!(offsets.off_weights % 64, 0);
    assert_eq!(offsets.off_dtimers % 64, 0);
    assert_eq!(offsets.total_state_size % 64, 0);

    assert_eq!(calculate_state_blob_size(64), 74688);
}

#[test]
fn test_paths_file_layout_math() {
    assert_eq!(calculate_paths_matrix_offset(0), 64);
    assert_eq!(calculate_paths_file_size(0), 64);

    assert_eq!(calculate_paths_matrix_offset(100), 256);
    assert_eq!(calculate_paths_file_size(100), 102656);
}

#[test]
fn test_vram_ptrs_layout() {
    let ptr_size = size_of::<*mut ()>();
    let ptr_align = align_of::<*mut ()>();

    assert_eq!(size_of::<ShardVramPtrs>(), 9 * ptr_size);
    assert_eq!(align_of::<ShardVramPtrs>(), ptr_align);

    assert_eq!(offset_of!(ShardVramPtrs, soma_voltage), 0);
    assert_eq!(offset_of!(ShardVramPtrs, soma_flags), ptr_size);
    assert_eq!(offset_of!(ShardVramPtrs, threshold_offset), 2 * ptr_size);
    assert_eq!(offset_of!(ShardVramPtrs, timers), 3 * ptr_size);
    assert_eq!(offset_of!(ShardVramPtrs, soma_to_axon), 4 * ptr_size);
    assert_eq!(offset_of!(ShardVramPtrs, dendrite_targets), 5 * ptr_size);
    assert_eq!(offset_of!(ShardVramPtrs, dendrite_weights), 6 * ptr_size);
    assert_eq!(offset_of!(ShardVramPtrs, dendrite_timers), 7 * ptr_size);
    assert_eq!(offset_of!(ShardVramPtrs, axon_heads), 8 * ptr_size);
}
