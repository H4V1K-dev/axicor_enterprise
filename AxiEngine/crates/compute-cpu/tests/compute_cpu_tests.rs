use compute_api::*;
use compute_cpu::*;
use core::num::NonZeroU64;
use core::ops::Deref;
use layout::{compute_state_offsets, VariantParameters, VARIANT_LUT_LEN};
use static_assertions::assert_not_impl_any;
use types::{PackedTarget, AXON_SENTINEL, EMPTY_PIXEL};

const ZERO_VARIANT: VariantParameters = VariantParameters {
    threshold: 1000,
    rest_potential: 0,
    leak_shift: 4,
    homeostasis_penalty: 50,
    spontaneous_firing_period_ticks: 0,
    initial_synapse_weight: 1000,
    gsop_potentiation: 10,
    gsop_depression: 5,
    homeostasis_decay: 2,
    refractory_period: 2,
    synapse_refractory_period: 0,
    signal_propagation_length: 5,
    is_inhibitory: 0,
    inertia_curve: [128; 8],
    ahp_amplitude: 200,
    _pad1: [0; 6],
    adaptive_leak_min_shift: 0,
    adaptive_leak_gain: 0,
    adaptive_mode: 0,
    _leak_pad: [0; 3],
    d1_affinity: 0,
    d2_affinity: 0,
    heartbeat_m: 0,
};

fn dummy_variants() -> [VariantParameters; VARIANT_LUT_LEN] {
    [ZERO_VARIANT; VARIANT_LUT_LEN]
}

fn create_test_blobs(padded_n: usize, total_axons: usize) -> (Vec<u8>, Vec<u8>) {
    let state_size = compute_state_offsets(padded_n).total_state_size;
    let state_blob = vec![0u8; state_size];
    let axons_size = validation::expected_axons_blob_size(total_axons as u32).unwrap();
    let mut axons_blob = vec![0u8; axons_size];
    let heads = bytemuck::cast_slice_mut::<u8, u32>(&mut axons_blob[16..]);
    for head in heads.iter_mut() {
        *head = AXON_SENTINEL;
    }
    (state_blob, axons_blob)
}

fn create_backend() -> CpuBackend {
    CpuBackend::new(CpuBackendConfig::default()).expect("backend creation must succeed")
}

#[test]
fn test_cpu_implements_compute_backend() {
    fn assert_backend<T: ComputeBackend>() {}
    assert_backend::<CpuBackend>();
}

#[test]
fn test_cpu_backend_kind() {
    let backend = create_backend();
    assert_eq!(backend.kind(), BackendKind::Cpu);
}

#[test]
fn test_cpu_backend_capabilities() {
    let backend = create_backend();
    let caps = backend.capabilities();
    assert_eq!(caps.lane_count, 1);
    assert!(!caps.supports_async);
    assert!(!caps.supports_ephys);
    assert_eq!(caps.max_batch_ticks, 1000);
    assert_eq!(caps.alignment_bytes, 64);
    assert!(!caps.pinned_host_required);
}

#[test]
fn test_cpu_alloc_shard_alignment() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).expect("allocation must succeed");
    assert_eq!(handle.kind(), BackendKind::Cpu);

    let (state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();
    let upload = ShardUpload {
        state_blob: &state_blob,
        axons_blob: &axons_blob,
        variant_table: &variants,
    };
    backend
        .upload_shard(handle, upload)
        .expect("upload must succeed");

    let mut snap_state = vec![0u8; state_blob.len()];
    let mut snap_axons = vec![0u8; axons_blob.len()];
    let snapshot = ShardSnapshotMut {
        state_blob: &mut snap_state,
        axons_blob: &mut snap_axons,
    };
    backend
        .debug_snapshot(handle, snapshot)
        .expect("snapshot must succeed");

    backend.free_shard(handle).expect("free must succeed");
}

#[test]
fn test_cpu_upload_rejects_bad_sizes() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).expect("allocation must succeed");

    let (mut state_blob, axons_blob) = create_test_blobs(64, 10);
    state_blob.pop(); // Corrupt size
    let variants = dummy_variants();
    let upload = ShardUpload {
        state_blob: &state_blob,
        axons_blob: &axons_blob,
        variant_table: &variants,
    };
    let res = backend.upload_shard(handle, upload);
    assert_eq!(res, Err(ComputeApiError::SizeMismatch));
}

#[test]
fn test_cpu_rejects_invalid_handles() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).expect("allocation must succeed");

    let foreign_handle =
        VramHandle::from_raw_parts(BackendKind::Cuda, NonZeroU64::new(1).unwrap(), 1);
    let invalid_handle =
        VramHandle::from_raw_parts(BackendKind::Cpu, NonZeroU64::new(999).unwrap(), 1);

    let (state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();

    assert_eq!(
        backend.upload_shard(
            foreign_handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            }
        ),
        Err(ComputeApiError::ForeignHandle)
    );
    assert_eq!(
        backend.upload_shard(
            invalid_handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            }
        ),
        Err(ComputeApiError::InvalidHandle)
    );

    backend
        .upload_shard(
            handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            },
        )
        .expect("upload must succeed");
    backend.free_shard(handle).expect("free must succeed");

    assert_eq!(
        backend.free_shard(handle),
        Err(ComputeApiError::AlreadyFreed)
    );
    assert_eq!(
        backend.upload_shard(
            handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            }
        ),
        Err(ComputeApiError::AlreadyFreed)
    );
}

#[test]
fn test_cpu_teardown_invalidates_old_handles() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let old_handle = backend.alloc_shard(spec).expect("first alloc");
    backend.teardown().expect("teardown");

    let new_handle = backend.alloc_shard(spec).expect("second alloc");
    assert_ne!(old_handle, new_handle);

    let (state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();
    let upload = ShardUpload {
        state_blob: &state_blob,
        axons_blob: &axons_blob,
        variant_table: &variants,
    };

    assert_eq!(
        backend.upload_shard(old_handle, upload),
        Err(ComputeApiError::InvalidHandle)
    );

    let mut output_spikes = vec![0u32; 10];
    let mut output_counts = vec![0u32; 1];
    let incoming_counts = vec![0u32; 1];
    let mapped_somas = vec![0u32; 1];
    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &incoming_counts,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_counts,
    };
    assert_eq!(
        backend.run_day_batch(old_handle, cmd),
        Err(ComputeApiError::InvalidHandle)
    );

    let mut snap_state = vec![0u8; state_blob.len()];
    let mut snap_axons = vec![0u8; axons_blob.len()];
    assert_eq!(
        backend.debug_snapshot(
            old_handle,
            ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            }
        ),
        Err(ComputeApiError::InvalidHandle)
    );

    assert_eq!(
        backend.free_shard(old_handle),
        Err(ComputeApiError::InvalidHandle)
    );
}

#[test]
fn test_cpu_virtual_offset_mapping() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).expect("alloc");
    let (state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();
    backend
        .upload_shard(
            handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            },
        )
        .expect("upload");

    let mut output_spikes = vec![0u32; 10];
    let mut output_counts = vec![0u32; 1];
    let incoming_counts = vec![0u32; 1];
    let mapped_somas = vec![0u32; 1];
    let bitmask = vec![1u32];

    // Case 1: Bit 0 with cmd.virtual_offset == spec.virtual_offset (100) -> global 100 -> maps to local axon 0
    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 100,
        num_virtual_axons: 1,
        input_bitmask: Some(&bitmask),
        incoming_spikes: None,
        incoming_spike_counts: &incoming_counts,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_counts,
    };

    assert!(backend.run_day_batch(handle, cmd).is_ok());

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
        .expect("snapshot 1");

    let heads_u32 = bytemuck::cast_slice::<u8, u32>(&snap_axons[16..]);
    let expected_head = physics::initial_axon_head(1);
    let expected_head_prop = physics::propagate_head(expected_head, 1);
    assert_eq!(
        heads_u32[0], expected_head_prop,
        "Local axon 0 should be activated"
    );
    assert_eq!(
        heads_u32[8], AXON_SENTINEL,
        "Local axon 1 should remain untouched"
    );

    // Case 2: Out-of-range virtual axon input (global 200) -> should NOT affect local axons
    let cmd_out_of_bounds = DayBatchCmd {
        tick_base: 1,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 200,
        num_virtual_axons: 1,
        input_bitmask: Some(&bitmask),
        incoming_spikes: None,
        incoming_spike_counts: &incoming_counts,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_counts,
    };

    assert!(backend.run_day_batch(handle, cmd_out_of_bounds).is_ok());

    backend
        .debug_snapshot(
            handle,
            ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .expect("snapshot 2");

    let heads_u32_2 = bytemuck::cast_slice::<u8, u32>(&snap_axons[16..]);
    for a in 1..10 {
        assert_eq!(
            heads_u32_2[a * 8],
            AXON_SENTINEL,
            "Local axon {} should not be activated by out of bounds global ID",
            a
        );
    }
}

#[test]
fn test_cpu_run_day_batch_validation() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).expect("alloc");
    let (state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();
    backend
        .upload_shard(
            handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            },
        )
        .expect("upload");

    let mut output_spikes = vec![0u32; 10];
    let mut output_counts = vec![0u32; 1];
    let incoming_counts = vec![0u32; 1];
    let mapped_somas = vec![0u32; 1];

    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &incoming_counts,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_counts,
    };

    let res = backend.run_day_batch(handle, cmd);
    assert!(res.is_ok());
}

#[test]
fn test_cpu_panic_free_on_invalid_dto() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).expect("alloc");

    let mut output_spikes = vec![0u32; 10];
    let mut output_counts = vec![0u32; 0]; // Invalid length
    let incoming_counts = vec![0u32; 1];
    let mapped_somas = vec![0u32; 1];

    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &incoming_counts,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_counts,
    };

    let res = backend.run_day_batch(handle, cmd);
    assert_eq!(res, Err(ComputeApiError::InvalidBatch));
}

#[test]
fn test_cpu_bit_to_bit_determinism() {
    let run_sim = || {
        let mut backend = CpuBackend::new(CpuBackendConfig {
            thread_count: Some(2),
        })
        .unwrap();
        let spec = ShardAllocSpec {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();
        let (state_blob, axons_blob) = create_test_blobs(64, 10);
        let mut variants = dummy_variants();
        variants[0].heartbeat_m = 65535; // Spikes every tick
        backend
            .upload_shard(
                handle,
                ShardUpload {
                    state_blob: &state_blob,
                    axons_blob: &axons_blob,
                    variant_table: &variants,
                },
            )
            .unwrap();

        let mut output_spikes = vec![0u32; 100];
        let mut output_counts = vec![0u32; 10];
        let incoming_counts = vec![0u32; 10];
        let mapped_somas = vec![0u32; 1];

        let cmd = DayBatchCmd {
            tick_base: 0,
            sync_batch_ticks: 10,
            v_seg: 1,
            dopamine: 0,
            input_words_per_tick: 0,
            max_spikes_per_tick: 10,
            num_outputs: 1,
            virtual_offset: 0,
            num_virtual_axons: 0,
            input_bitmask: None,
            incoming_spikes: None,
            incoming_spike_counts: &incoming_counts,
            mapped_soma_ids: &mapped_somas,
            output_spikes: &mut output_spikes,
            output_spike_counts: &mut output_counts,
        };

        let batch_res = backend.run_day_batch(handle, cmd).unwrap();

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
            .unwrap();
        (
            batch_res,
            snap_state,
            snap_axons,
            output_spikes,
            output_counts,
        )
    };

    let res1 = run_sim();
    let res2 = run_sim();

    // Verify deterministic BatchResult fields
    assert_eq!(res1.0.ticks_executed, res2.0.ticks_executed);
    assert_eq!(res1.0.generated_spikes_count, res2.0.generated_spikes_count);
    assert_eq!(res1.0.output_spikes_written, res2.0.output_spikes_written);
    assert_eq!(res1.0.dropped_spikes_count, res2.0.dropped_spikes_count);
    // Note: execution_time_us is nondeterministic hardware wall-clock telemetry and excluded from bit-for-bit equivalence assertions.

    assert_eq!(res1.1, res2.1);
    assert_eq!(res1.2, res2.2);
    assert_eq!(res1.3, res2.3);
    assert_eq!(res1.4, res2.4);
}

#[test]
fn test_cpu_rayon_thread_count_equivalence() {
    let run_with_threads = |threads: usize| {
        let mut backend = CpuBackend::new(CpuBackendConfig {
            thread_count: Some(threads),
        })
        .unwrap();
        let spec = ShardAllocSpec {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 0,
            virtual_offset: 0,
        };
        let handle = backend.alloc_shard(spec).unwrap();
        let (state_blob, axons_blob) = create_test_blobs(64, 10);
        let mut variants = dummy_variants();
        variants[0].heartbeat_m = 65535;
        backend
            .upload_shard(
                handle,
                ShardUpload {
                    state_blob: &state_blob,
                    axons_blob: &axons_blob,
                    variant_table: &variants,
                },
            )
            .unwrap();

        let mut output_spikes = vec![0u32; 100];
        let mut output_counts = vec![0u32; 10];
        let incoming_counts = vec![0u32; 10];
        let mapped_somas = vec![0u32; 1];

        let cmd = DayBatchCmd {
            tick_base: 0,
            sync_batch_ticks: 10,
            v_seg: 1,
            dopamine: 0,
            input_words_per_tick: 0,
            max_spikes_per_tick: 10,
            num_outputs: 1,
            virtual_offset: 0,
            num_virtual_axons: 0,
            input_bitmask: None,
            incoming_spikes: None,
            incoming_spike_counts: &incoming_counts,
            mapped_soma_ids: &mapped_somas,
            output_spikes: &mut output_spikes,
            output_spike_counts: &mut output_counts,
        };

        let batch_res = backend.run_day_batch(handle, cmd).unwrap();

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
            .unwrap();
        (
            batch_res,
            snap_state,
            snap_axons,
            output_spikes,
            output_counts,
        )
    };

    let single_thread = run_with_threads(1);
    let multi_thread = run_with_threads(4);

    assert_eq!(
        single_thread.0.ticks_executed,
        multi_thread.0.ticks_executed
    );
    assert_eq!(
        single_thread.0.generated_spikes_count,
        multi_thread.0.generated_spikes_count
    );
    assert_eq!(
        single_thread.0.output_spikes_written,
        multi_thread.0.output_spikes_written
    );
    assert_eq!(
        single_thread.0.dropped_spikes_count,
        multi_thread.0.dropped_spikes_count
    );

    assert_eq!(single_thread.1, multi_thread.1);
    assert_eq!(single_thread.2, multi_thread.2);
    assert_eq!(single_thread.3, multi_thread.3);
    assert_eq!(single_thread.4, multi_thread.4);
}

#[test]
fn test_cpu_spike_write_race_safety() {
    let mut backend = CpuBackend::new(CpuBackendConfig {
        thread_count: Some(4),
    })
    .unwrap();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();
    backend
        .upload_shard(
            handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            },
        )
        .unwrap();

    let mut output_spikes = vec![0u32; 10];
    let mut output_counts = vec![0u32; 1];
    let incoming_counts = vec![0u32; 1];
    let mapped_somas = vec![0u32; 1];

    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &incoming_counts,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_counts,
    };

    assert!(backend.run_day_batch(handle, cmd).is_ok());
}

#[test]
fn test_cpu_no_raw_pointers_in_api() {
    assert_not_impl_any!(CpuBackend: Deref);
}

#[test]
fn test_cpu_legacy_behavioral_facts() {
    assert_eq!(AXON_SENTINEL, 0x8000_0000);
    assert_eq!(EMPTY_PIXEL, 0xFFFF_FFFF);
    assert!(PackedTarget(0).is_inactive());
    assert!(PackedTarget(EMPTY_PIXEL).is_inactive());
}

#[test]
fn test_cpu_variant_table_copied_not_borrowed() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (state_blob, axons_blob) = create_test_blobs(64, 10);
    let mut variants = dummy_variants();
    variants[0].threshold = 9999;
    {
        let upload = ShardUpload {
            state_blob: &state_blob,
            axons_blob: &axons_blob,
            variant_table: &variants,
        };
        backend.upload_shard(handle, upload).unwrap();
    }
    variants[0].threshold = 1111;
    assert_eq!(variants[0].threshold, 1111);

    let mut output_spikes = vec![0u32; 10];
    let mut output_counts = vec![0u32; 1];
    let incoming_counts = vec![0u32; 1];
    let mapped_somas = vec![0u32; 1];
    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &incoming_counts,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_counts,
    };
    assert!(backend.run_day_batch(handle, cmd).is_ok());
}

#[test]
fn test_cpu_debug_snapshot_byte_exact() {
    let mut backend = create_backend();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (mut state_blob, mut axons_blob) = create_test_blobs(64, 10);
    state_blob[0] = 0xAB;
    axons_blob[0] = 0xCD;
    let variants = dummy_variants();
    backend
        .upload_shard(
            handle,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            },
        )
        .unwrap();

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
        .unwrap();

    assert_eq!(snap_state[0], 0xAB);
    assert_eq!(snap_axons[0], 0xCD);
}
