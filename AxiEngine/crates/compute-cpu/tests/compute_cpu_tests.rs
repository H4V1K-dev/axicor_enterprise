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
    fatigue_capacity: 255,
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

#[test]
fn test_stochastic_heartbeat_somatic_reset_cost() {
    let mut backend = create_backend();

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (mut state_blob, axons_blob) = create_test_blobs(64, 10);
    let mut variants = dummy_variants();
    // Variant 0 has heartbeat enabled
    variants[0].heartbeat_m = physics::MAX_HEARTBEAT_M;
    variants[0].refractory_period = 5;
    variants[0].ahp_amplitude = 200;
    variants[0].homeostasis_penalty = 100;
    // Variant 1 has heartbeat disabled
    variants[1].heartbeat_m = 0;

    let offsets = layout::compute_state_offsets(64);
    // Assign soma 0 -> variant 0, somas 1..64 -> variant 1
    let flags_slice = &mut state_blob[offsets.off_flags..offsets.off_thresh];
    for f in flags_slice.iter_mut().skip(1) {
        *f = types::SomaFlags::new(false, 0, 1).0;
    }

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

    let mut output_spikes = [0u32; 64];
    let mut output_spike_counts = [0u32; 1];
    let mapped_soma_ids = [0u32];
    let incoming_spike_counts = [0u32];

    let cmd = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 100,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: None,
        incoming_spike_counts: &incoming_spike_counts,
        max_spikes_per_tick: 64,
        num_outputs: 1,
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    let res = backend.run_day_batch(handle, cmd).unwrap();
    assert_eq!(res.generated_spikes_count, 1);
    assert_eq!(output_spike_counts[0], 1);

    // Verify Somatic Reset Cost via debug_snapshot
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

    let soma_voltage =
        bytemuck::cast_slice::<u8, i32>(&snap_state[offsets.off_voltage..offsets.off_flags]);
    let timers = &snap_state[offsets.off_timers..offsets.off_s2a];
    let thresh_offset =
        bytemuck::cast_slice::<u8, i32>(&snap_state[offsets.off_thresh..offsets.off_timers]);

    assert_eq!(soma_voltage[0], variants[0].rest_potential - 200);
    assert_eq!(timers[0], 5);
    assert_eq!(thresh_offset[0], 100);
}

#[test]
fn test_dendrite_fatigue_accumulation_and_recovery() {
    let mut backend = create_backend();

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (mut state_blob, axons_blob) = create_test_blobs(64, 10);
    let mut variants = dummy_variants();
    variants[0].fatigue_capacity = 255;
    variants[0].refractory_period = 0; // Keep soma non-refractory for test

    let offsets = layout::compute_state_offsets(64);
    // Wire dendrite slot 0 of soma 0 to axon 1, seg 0
    let packed_target = types::PackedTarget::pack(1, 0).0;
    let targets_slice = bytemuck::cast_slice_mut::<u8, u32>(
        &mut state_blob[offsets.off_targets..offsets.off_weights],
    );
    targets_slice[0] = packed_target;

    let weights_slice = bytemuck::cast_slice_mut::<u8, i32>(
        &mut state_blob[offsets.off_weights..offsets.off_dtimers],
    );
    weights_slice[0] = 100_000;

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

    // Inject incoming spike on axon 1
    let mut incoming_spikes = [0u32; 64];
    incoming_spikes[0] = 1;
    let incoming_spike_counts = [1u32];
    let mut output_spikes = [0u32; 64];
    let mut output_spike_counts = [0u32; 1];

    let cmd = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 10,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: Some(&incoming_spikes),
        incoming_spike_counts: &incoming_spike_counts,
        max_spikes_per_tick: 64,
        num_outputs: 0,
        mapped_soma_ids: &[],
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    backend.run_day_batch(handle, cmd).unwrap();

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

    // Dendrite timer at slot 0 should have accumulated FATIGUE_SPIKE_COST (50) on tick 1 hit
    let dendrite_timers =
        &snap_state[offsets.off_dtimers..offsets.off_dtimers + 64 * layout::MAX_DENDRITES];
    assert_eq!(dendrite_timers[0], 50);

    // On tick 2, active tail of length 5 is still propagating over segment 0 -> 2nd hit adds +50 after 1 recovery tick (50 - 1 + 50 = 99)
    let cmd_idle = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 11,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        max_spikes_per_tick: 64,
        num_outputs: 0,
        mapped_soma_ids: &[],
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    backend.run_day_batch(handle, cmd_idle).unwrap();

    backend
        .debug_snapshot(
            handle,
            ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    let dendrite_timers =
        &snap_state[offsets.off_dtimers..offsets.off_dtimers + 64 * layout::MAX_DENDRITES];
    assert_eq!(dendrite_timers[0], 99);
}

#[test]
fn test_stochastic_heartbeat_deterministic_replay() {
    let mut backend1 = create_backend();
    let mut backend2 = create_backend();

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle1 = backend1.alloc_shard(spec).unwrap();
    let handle2 = backend2.alloc_shard(spec).unwrap();

    let (state_blob, axons_blob) = create_test_blobs(64, 10);
    let mut variants = dummy_variants();
    variants[0].heartbeat_m = 30000; // ~45% probability per tick

    backend1
        .upload_shard(
            handle1,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            },
        )
        .unwrap();

    backend2
        .upload_shard(
            handle2,
            ShardUpload {
                state_blob: &state_blob,
                axons_blob: &axons_blob,
                variant_table: &variants,
            },
        )
        .unwrap();

    let mut out_spikes1 = [0u32; 64];
    let mut out_counts1 = [0u32; 1];
    let mut out_spikes2 = [0u32; 64];
    let mut out_counts2 = [0u32; 1];
    let mapped_somas = [0u32];

    let cmd1 = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 500,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        max_spikes_per_tick: 64,
        num_outputs: 1,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut out_spikes1,
        output_spike_counts: &mut out_counts1,
    };

    let cmd2 = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 500,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        max_spikes_per_tick: 64,
        num_outputs: 1,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut out_spikes2,
        output_spike_counts: &mut out_counts2,
    };

    let res1 = backend1.run_day_batch(handle1, cmd1).unwrap();
    let res2 = backend2.run_day_batch(handle2, cmd2).unwrap();

    assert_eq!(res1.generated_spikes_count, res2.generated_spikes_count);
    assert_eq!(out_counts1[0], out_counts2[0]);
    assert_eq!(out_spikes1, out_spikes2);
}

#[test]
fn test_inactive_dendrite_slot_reset() {
    let mut backend = create_backend();

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (mut state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();

    let offsets = layout::compute_state_offsets(64);
    // Explicitly set an inactive target with non-zero fatigue timer (100)
    let targets_slice = bytemuck::cast_slice_mut::<u8, u32>(
        &mut state_blob[offsets.off_targets..offsets.off_weights],
    );
    targets_slice[0] = types::PackedTarget::NONE.0; // Inactive target

    let timers_slice =
        &mut state_blob[offsets.off_dtimers..offsets.off_dtimers + 64 * layout::MAX_DENDRITES];
    timers_slice[0] = 100; // Stale fatigue value

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

    let mut output_spikes = [0u32; 64];
    let mut output_spike_counts = [0u32; 1];

    let cmd = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 10,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        max_spikes_per_tick: 64,
        num_outputs: 0,
        mapped_soma_ids: &[],
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    backend.run_day_batch(handle, cmd).unwrap();

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

    // Verify inactive slot fatigue was reset to 0
    let dendrite_timers =
        &snap_state[offsets.off_dtimers..offsets.off_dtimers + 64 * layout::MAX_DENDRITES];
    assert_eq!(dendrite_timers[0], 0);
}

#[test]
fn test_dendrite_fatigue_attenuation_forward_pass() {
    // Tests that higher dendrite fatigue attenuates input weight during forward pass
    let mut backend = create_backend();

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (mut state_blob, axons_blob) = create_test_blobs(64, 10);
    let mut variants = dummy_variants();
    variants[0].fatigue_capacity = 200; // Capacity 200

    let offsets = layout::compute_state_offsets(64);

    // Setup 2 somas with active dendritic input from axon 0 at segment 1
    let targets_slice = bytemuck::cast_slice_mut::<u8, u32>(
        &mut state_blob[offsets.off_targets..offsets.off_weights],
    );
    let packed_target = types::PackedTarget::pack(0, 0).0; // Axon 0, segment 0
    targets_slice[0] = packed_target; // Soma 0 slot 0
    targets_slice[1] = packed_target; // Soma 1 slot 0

    let weights_slice = bytemuck::cast_slice_mut::<u8, i32>(
        &mut state_blob[offsets.off_weights..offsets.off_dtimers],
    );
    weights_slice[0] = 6_553_600; // 100 in charge domain (100 << 16)
    weights_slice[1] = 6_553_600; // 100 in charge domain

    let timers_slice =
        &mut state_blob[offsets.off_dtimers..offsets.off_dtimers + 64 * layout::MAX_DENDRITES];
    timers_slice[0] = 0; // Soma 0: No fatigue -> full 100 input
    timers_slice[1] = 100; // Soma 1: 50% fatigue (100/200) -> 50 input

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

    // Inject spike into virtual axon 0
    let mut bitmask = vec![0u32; 1];
    bitmask[0] = 1; // Spiking axon 0
    let mapped_somas = vec![0u32; 1];
    let mut out_spikes = vec![0u32; 64];
    let mut out_counts = vec![0u32; 1];

    let cmd = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 0,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: Some(&bitmask),
        num_virtual_axons: 1,
        virtual_offset: 0,
        input_words_per_tick: 1,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        max_spikes_per_tick: 64,
        num_outputs: 1,
        mapped_soma_ids: &mapped_somas,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };

    backend.run_day_batch(handle, cmd).unwrap();

    backend
        .debug_snapshot(
            handle,
            ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    let voltage_slice =
        bytemuck::cast_slice::<u8, i32>(&snap_state[offsets.off_voltage..offsets.off_flags]);
    // Soma 0 voltage should be rest + 100 = 100
    // Soma 1 voltage should be rest + 50 = 50
    assert_eq!(voltage_slice[0], 100);
    assert_eq!(voltage_slice[1], 50);

    // Also check fatigue accumulation after spike hit:
    // Soma 0 fatigue went 0 -> min(0 + 50, 200) = 50
    // Soma 1 fatigue went 100 - 1 (recovery) + 50 = 149
    let dendrite_timers =
        &snap_state[offsets.off_dtimers..offsets.off_dtimers + 64 * layout::MAX_DENDRITES];
    assert_eq!(dendrite_timers[0], 50);
    assert_eq!(dendrite_timers[1], 149);
}

#[test]
fn test_dendrite_fatigue_recovery_during_soma_refractory() {
    // Verifies that dendritic fatigue recovers (-1/tick) even when soma is in refractory period
    let mut backend = create_backend();

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (mut state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();

    let offsets = layout::compute_state_offsets(64);
    let targets_slice = bytemuck::cast_slice_mut::<u8, u32>(
        &mut state_blob[offsets.off_targets..offsets.off_weights],
    );
    targets_slice[0] = types::PackedTarget::pack(0, 1).0; // Target slot 0

    // Put soma in refractory period (timer = 5)
    state_blob[offsets.off_timers] = 5;

    // Set dendrite timer = 40
    let timers_slice =
        &mut state_blob[offsets.off_dtimers..offsets.off_dtimers + 64 * layout::MAX_DENDRITES];
    timers_slice[0] = 40;

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

    let mut out_spikes = vec![0u32; 64];
    let mut out_counts = vec![0u32; 1];

    let cmd = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 0,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        max_spikes_per_tick: 64,
        num_outputs: 0,
        mapped_soma_ids: &[],
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };

    backend.run_day_batch(handle, cmd).unwrap();

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

    // Verify soma timer decremented 5 -> 4
    assert_eq!(snap_state[offsets.off_timers], 4);

    // Verify dendrite fatigue decremented 40 -> 39
    let dendrite_timers =
        &snap_state[offsets.off_dtimers..offsets.off_dtimers + 64 * layout::MAX_DENDRITES];
    assert_eq!(dendrite_timers[0], 39);
}

#[test]
fn test_cpu_soma_order_independence() {
    // Verifies that a spike from Soma A (offset 0) to Soma B (offset 1) is NOT integrated
    // in the same tick (proving order independence), but is integrated in the next tick after propagation.
    let mut backend = create_backend();

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    let (mut state_blob, axons_blob) = create_test_blobs(64, 10);
    let variants = dummy_variants();

    let offsets = layout::compute_state_offsets(64);

    // 1. Map somas to axons: Soma 0 -> Axon 0, Soma 1 -> Axon 1
    let s2a_slice =
        bytemuck::cast_slice_mut::<u8, u32>(&mut state_blob[offsets.off_s2a..offsets.off_targets]);
    for i in 0..64 {
        s2a_slice[i] = i as u32;
    }

    // 2. Connect Soma 1's dendrite slot 0 to target Axon 0, segment 0
    let targets_slice = bytemuck::cast_slice_mut::<u8, u32>(
        &mut state_blob[offsets.off_targets..offsets.off_weights],
    );
    targets_slice[1] = types::PackedTarget::pack(0, 0).0; // Soma 1 (index 1) target slot 0

    // 3. Set weight of Soma 1's dendrite slot 0 to 2000 << 16 (high weight)
    let weights_slice = bytemuck::cast_slice_mut::<u8, i32>(
        &mut state_blob[offsets.off_weights..offsets.off_dtimers],
    );
    weights_slice[1] = 2000 << 16;

    // 4. Initialize Soma 0 voltage to 2000 (well above threshold 1000 so it spikes immediately)
    let voltage_slice = bytemuck::cast_slice_mut::<u8, i32>(
        &mut state_blob[offsets.off_voltage..offsets.off_flags],
    );
    voltage_slice[0] = 2000;
    voltage_slice[1] = 0; // Soma 1 starts at 0

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

    let mut out_spikes = vec![0u32; 64];
    let mut out_counts = vec![0u32; 1];

    // --- Tick 0 ---
    let cmd1 = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 0,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        max_spikes_per_tick: 64,
        num_outputs: 0,
        mapped_soma_ids: &[],
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };

    backend.run_day_batch(handle, cmd1).unwrap();

    let mut snap1_state = vec![0u8; state_blob.len()];
    let mut snap1_axons = vec![0u8; axons_blob.len()];
    backend
        .debug_snapshot(
            handle,
            ShardSnapshotMut {
                state_blob: &mut snap1_state,
                axons_blob: &mut snap1_axons,
            },
        )
        .unwrap();

    let flags1 = &snap1_state[offsets.off_flags..offsets.off_thresh];
    let v1 = bytemuck::cast_slice::<u8, i32>(&snap1_state[offsets.off_voltage..offsets.off_flags]);

    // Soma 0 should have spiked in Tick 0
    assert!(
        types::SomaFlags(flags1[0]).spiking(),
        "Soma 0 should spike in Tick 0"
    );

    // Soma 1 should NOT have spiked in Tick 0 (order independence)
    assert!(
        !types::SomaFlags(flags1[1]).spiking(),
        "Soma 1 must not spike in the same tick"
    );
    assert_eq!(
        v1[1], 0,
        "Soma 1 voltage must remain unaffected in the same tick"
    );

    // --- Tick 1 ---
    let mut out_spikes2 = vec![0u32; 64];
    let mut out_counts2 = vec![0u32; 1];
    let cmd2 = DayBatchCmd {
        sync_batch_ticks: 1,
        tick_base: 1,
        v_seg: 1,
        dopamine: 0,
        input_bitmask: None,
        num_virtual_axons: 0,
        virtual_offset: 0,
        input_words_per_tick: 0,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        max_spikes_per_tick: 64,
        num_outputs: 0,
        mapped_soma_ids: &[],
        output_spikes: &mut out_spikes2,
        output_spike_counts: &mut out_counts2,
    };

    backend.run_day_batch(handle, cmd2).unwrap();

    let mut snap2_state = vec![0u8; state_blob.len()];
    let mut snap2_axons = vec![0u8; axons_blob.len()];
    backend
        .debug_snapshot(
            handle,
            ShardSnapshotMut {
                state_blob: &mut snap2_state,
                axons_blob: &mut snap2_axons,
            },
        )
        .unwrap();

    let flags2 = &snap2_state[offsets.off_flags..offsets.off_thresh];
    let v2 = bytemuck::cast_slice::<u8, i32>(&snap2_state[offsets.off_voltage..offsets.off_flags]);

    // Soma 1 should have spiked in Tick 1 after receiving the propagated spike from Soma 0
    assert!(
        types::SomaFlags(flags2[1]).spiking(),
        "Soma 1 should spike in Tick 1"
    );
    // After spiking, Soma 1 voltage is reset to: rest_potential (0) - ahp_amplitude (200) = -200
    assert_eq!(v2[1], -200, "Soma 1 voltage should be reset post-spike");
}

#[test]
fn test_cpu_maintenance_roundtrip() {
    let mut backend = CpuBackend::new(CpuBackendConfig { thread_count: Some(1) }).unwrap();
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

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = validation::expected_axons_blob_size(10).unwrap();

    let mut state_exp = vec![0u8; state_size];
    let mut axons_exp = vec![0u8; axons_size];

    // 1. Export maintenance state
    backend
        .export_maintenance_state(
            handle,
            BackendMaintenanceMut {
                state_blob: &mut state_exp,
                axons_blob: &mut axons_exp,
            },
        )
        .unwrap();

    // Verify it matches what we uploaded
    assert_eq!(state_exp, state_blob);
    assert_eq!(axons_exp, axons_blob);

    // 2. Modify state_exp
    let offsets = layout::compute_state_offsets(64);
    {
        let voltage_slice = bytemuck::cast_slice_mut::<u8, i32>(
            &mut state_exp[offsets.off_voltage..offsets.off_flags],
        );
        voltage_slice[0] = 5000;
    }

    // 3. Import maintenance state
    backend
        .import_maintenance_state(
            handle,
            BackendMaintenanceRef {
                state_blob: &state_exp,
                axons_blob: &axons_exp,
            },
        )
        .unwrap();

    // 4. Verify via debug_snapshot that voltage is now 5000
    let mut snap_state = vec![0u8; state_size];
    let mut snap_axons = vec![0u8; axons_size];
    backend
        .debug_snapshot(
            handle,
            ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    let voltage_slice = bytemuck::cast_slice::<u8, i32>(&snap_state[offsets.off_voltage..offsets.off_flags]);
    assert_eq!(voltage_slice[0], 5000);

    // 5. Test size mismatch rejection
    let mut bad_state_exp = vec![0u8; state_size + 1];
    let res = backend.export_maintenance_state(
        handle,
        BackendMaintenanceMut {
            state_blob: &mut bad_state_exp,
            axons_blob: &mut axons_exp,
        },
    );
    assert_eq!(res, Err(ComputeApiError::SizeMismatch));

    let bad_state_ref = vec![0u8; state_size - 1];
    let res = backend.import_maintenance_state(
        handle,
        BackendMaintenanceRef {
            state_blob: &bad_state_ref,
            axons_blob: &axons_exp,
        },
    );
    assert_eq!(res, Err(ComputeApiError::SizeMismatch));
}

