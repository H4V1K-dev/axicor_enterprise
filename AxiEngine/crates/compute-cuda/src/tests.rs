use super::*;

#[cfg(feature = "native")]
static GPU_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_cuda_implements_compute_backend() {
    fn assert_impl<T: ComputeBackend>() {}
    assert_impl::<CudaBackend>();
}

#[test]
fn test_cuda_backend_kind_compile_surface() {
    let backend = CudaBackend {
        _config: CudaBackendConfig::default(),
        registry: ResourceRegistry::default(),
        _marker: std::marker::PhantomData,
    };
    assert_eq!(backend.kind(), BackendKind::Cuda);
}

#[test]
fn test_cuda_static_capabilities() {
    let caps = CudaBackend::static_capabilities();
    assert_eq!(caps.lane_count, 32);
    assert!(caps.supports_async);
    assert!(!caps.supports_ephys);
    assert_eq!(caps.max_batch_ticks, 1000);
    assert_eq!(caps.alignment_bytes, 64);
    assert!(caps.pinned_host_required);
}

#[test]
fn test_cuda_is_not_send_sync() {
    static_assertions::assert_not_impl_any!(CudaBackend: Send, Sync);
}

#[test]
fn test_cuda_generated_abi_header_contains_expected_constants() {
    let header_content = include_str!(concat!(env!("OUT_DIR"), "/generated/axi_cuda_abi.h"));

    // Structure sizes and alignments
    assert!(header_content.contains(&format!(
        "#define AXI_SIZE_VariantParameters {}",
        std::mem::size_of::<layout::VariantParameters>()
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_ALIGN_VariantParameters {}",
        std::mem::align_of::<layout::VariantParameters>()
    )));

    assert!(header_content.contains(&format!(
        "#define AXI_SIZE_BurstHeads8 {}",
        std::mem::size_of::<layout::BurstHeads8>()
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_ALIGN_BurstHeads8 {}",
        std::mem::align_of::<layout::BurstHeads8>()
    )));

    assert!(header_content.contains(&format!(
        "#define AXI_SIZE_StateFileHeader {}",
        std::mem::size_of::<layout::StateFileHeader>()
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_ALIGN_StateFileHeader {}",
        std::mem::align_of::<layout::StateFileHeader>()
    )));

    assert!(header_content.contains(&format!(
        "#define AXI_SIZE_AxonsFileHeader {}",
        std::mem::size_of::<layout::AxonsFileHeader>()
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_ALIGN_AxonsFileHeader {}",
        std::mem::align_of::<layout::AxonsFileHeader>()
    )));

    assert!(header_content.contains(&format!(
        "#define AXI_SIZE_PathsFileHeader {}",
        std::mem::size_of::<layout::PathsFileHeader>()
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_ALIGN_PathsFileHeader {}",
        std::mem::align_of::<layout::PathsFileHeader>()
    )));

    assert!(header_content.contains(&format!(
        "#define AXI_SIZE_ShardVramPtrs {}",
        std::mem::size_of::<layout::ShardVramPtrs>()
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_ALIGN_ShardVramPtrs {}",
        std::mem::align_of::<layout::ShardVramPtrs>()
    )));

    // Layout constants
    assert!(header_content.contains(&format!(
        "#define AXI_CACHE_LINE_BYTES {}",
        layout::CACHE_LINE_BYTES
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_PADDED_N_ALIGNMENT {}",
        layout::PADDED_N_ALIGNMENT
    )));

    // Types and physics constants
    assert!(header_content.contains(&format!(
        "#define AXI_AXON_SENTINEL 0x{:08X}",
        types::AXON_SENTINEL
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_EMPTY_PIXEL 0x{:08X}",
        types::EMPTY_PIXEL
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_MIN_WEIGHT_LIMIT {}",
        physics::constants::MIN_WEIGHT_LIMIT
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_MAX_WEIGHT_LIMIT {}",
        physics::constants::MAX_WEIGHT_LIMIT
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_DDS_PHASE_MOD {}ULL",
        physics::constants::DDS_PHASE_MOD
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_DDS_PHASE_MASK 0x{:X}ULL",
        physics::constants::DDS_PHASE_MASK
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_DDS_SCATTER_PRIME {}ULL",
        physics::constants::DDS_SCATTER_PRIME
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_MAX_HEARTBEAT_M {}",
        physics::constants::MAX_HEARTBEAT_M
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_MAX_DENDRITES {}",
        layout::MAX_DENDRITES
    )));
    assert!(header_content.contains(&format!("#define AXI_MAX_AXON_ID {}", types::MAX_AXON_ID)));
    assert!(header_content.contains(&format!(
        "#define AXI_MAX_SEGMENT_OFFSET {}",
        types::MAX_SEGMENT_OFFSET
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_MASS_TO_CHARGE_SHIFT {}",
        physics::constants::MASS_TO_CHARGE_SHIFT
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_INERTIA_RANK_SHIFT {}",
        physics::constants::INERTIA_RANK_SHIFT
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_MAX_INERTIA_RANK {}",
        physics::constants::MAX_INERTIA_RANK
    )));

    let dummy = layout::VariantParameters {
        threshold: 0,
        rest_potential: 0,
        leak_shift: 0,
        homeostasis_penalty: 0,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 0,
        refractory_period: 0,
        synapse_refractory_period: 0,
        signal_propagation_length: 0,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let base_ptr = &dummy as *const _ as usize;
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_threshold {}",
        (&dummy.threshold as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_rest_potential {}",
        (&dummy.rest_potential as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_leak_shift {}",
        (&dummy.leak_shift as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_homeostasis_penalty {}",
        (&dummy.homeostasis_penalty as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_homeostasis_decay {}",
        (&dummy.homeostasis_decay as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_refractory_period {}",
        (&dummy.refractory_period as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_ahp_amplitude {}",
        (&dummy.ahp_amplitude as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_adaptive_leak_min_shift {}",
        (&dummy.adaptive_leak_min_shift as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_adaptive_leak_gain {}",
        (&dummy.adaptive_leak_gain as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_adaptive_mode {}",
        (&dummy.adaptive_mode as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_signal_propagation_length {}",
        (&dummy.signal_propagation_length as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_heartbeat_m {}",
        (&dummy.heartbeat_m as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_gsop_potentiation {}",
        (&dummy.gsop_potentiation as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_gsop_depression {}",
        (&dummy.gsop_depression as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_d1_affinity {}",
        (&dummy.d1_affinity as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_d2_affinity {}",
        (&dummy.d2_affinity as *const _ as usize) - base_ptr
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_OFFSET_VariantParameters_inertia_curve {}",
        (&dummy.inertia_curve as *const _ as usize) - base_ptr
    )));

    assert!(header_content.contains(&format!(
        "#define AXI_SOMA_SPIKING_MASK {}",
        types::SOMA_SPIKING_MASK
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_SOMA_BURST_MASK {}",
        types::SOMA_BURST_MASK
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_SOMA_BURST_SHIFT {}",
        types::SOMA_BURST_SHIFT
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_SOMA_TYPE_MASK {}",
        types::SOMA_TYPE_MASK
    )));
    assert!(header_content.contains(&format!(
        "#define AXI_SOMA_TYPE_SHIFT {}",
        types::SOMA_TYPE_SHIFT
    )));
}

#[test]
#[cfg(not(feature = "native"))]
fn test_cuda_new_without_native_returns_unsupported_backend() {
    let res = CudaBackend::new(CudaBackendConfig::default());
    assert!(matches!(res, Err(ComputeApiError::UnsupportedBackend)));
}

#[cfg(feature = "native")]
fn is_gpu_available() -> bool {
    unsafe { native::axi_cuda_probe_device(0) == 0 }
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_propagate_head() {
    if !is_gpu_available() {
        println!("CUDA GPU not available, skipping test.");
        return;
    }

    let cases = vec![
        // a) normal active
        (100, 5),
        // b) head = AXON_SENTINEL
        (types::AXON_SENTINEL, 5),
        // c) head = AXON_SENTINEL - 1, v_seg = 1
        (types::AXON_SENTINEL - 1, 1),
        // d) head = AXON_SENTINEL - 1, v_seg = 2
        (types::AXON_SENTINEL - 1, 2),
        // e) v_seg = 0
        (10, 0),
        // general active propagation and clamp limits
        (types::AXON_SENTINEL - 100, 50),
        (types::AXON_SENTINEL - 100, 150),
    ];

    for (head, v_seg) in cases {
        let gpu_res = CudaBackend::cuda_propagate_head_for_test(head, v_seg).unwrap();
        let cpu_res = physics::propagate_head(head, v_seg);
        assert_eq!(
            gpu_res, cpu_res,
            "propagate_head mismatch for head={}, v_seg={}",
            head, v_seg
        );
    }
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_active_tail_hit() {
    if !is_gpu_available() {
        println!("CUDA GPU not available, skipping test.");
        return;
    }

    let cases = vec![
        // a) d < propagation_length -> true
        (100, 95, 10),
        // b) d == propagation_length -> false
        (100, 90, 10),
        // c) d > propagation_length -> false
        (100, 80, 10),
        // d) head = AXON_SENTINEL -> false
        (types::AXON_SENTINEL, 10, 5),
        // e) wraparound case: head < seg_idx -> false
        (10, 20, 5),
        // wraparound case that crosses zero -> true
        (10, 5, 10),
    ];

    for (head, seg_idx, prop_len) in cases {
        let gpu_res = CudaBackend::cuda_active_tail_hit_for_test(head, seg_idx, prop_len).unwrap();

        let mut heads = [types::AXON_SENTINEL; 8];
        heads[0] = head;
        let cpu_res = physics::active_tail_hit(&heads, seg_idx, prop_len);

        assert_eq!(
            gpu_res, cpu_res,
            "active_tail_hit mismatch for head={}, seg_idx={}, prop_len={}",
            head, seg_idx, prop_len
        );
    }
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_alloc_shard_returns_cuda_handle() {
    if !is_gpu_available() {
        return;
    }
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    assert_eq!(handle.kind(), BackendKind::Cuda);
    assert_eq!(handle.generation(), 1);
    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_upload_and_debug_snapshot_byte_exact() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

    let mut test_state = vec![0u8; state_size];
    for (i, val) in test_state.iter_mut().enumerate() {
        *val = (i & 0xFF) as u8;
    }

    let mut test_axons = vec![0u8; axons_size];
    for (i, val) in test_axons.iter_mut().enumerate() {
        *val = ((i * 3 + 7) & 0xFF) as u8;
    }

    let const_zero_variant = layout::VariantParameters {
        threshold: 0,
        rest_potential: 0,
        leak_shift: 0,
        homeostasis_penalty: 0,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 0,
        refractory_period: 0,
        synapse_refractory_period: 0,
        signal_propagation_length: 0,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let variant_table = [const_zero_variant; layout::VARIANT_LUT_LEN];

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons,
        variant_table: &variant_table,
    };

    backend.upload_shard(handle, upload).unwrap();

    let mut snap_state = vec![0u8; state_size];
    let mut snap_axons = vec![0u8; axons_size];
    let snapshot = compute_api::ShardSnapshotMut {
        state_blob: &mut snap_state,
        axons_blob: &mut snap_axons,
    };

    backend.debug_snapshot(handle, snapshot).unwrap();

    assert_eq!(snap_state, test_state);
    assert_eq!(snap_axons, test_axons);

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_rejects_bad_upload_sizes() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

    let test_state = vec![0u8; state_size - 1]; // bad size
    let test_axons = vec![0u8; axons_size];

    let const_zero_variant = layout::VariantParameters {
        threshold: 0,
        rest_potential: 0,
        leak_shift: 0,
        homeostasis_penalty: 0,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 0,
        refractory_period: 0,
        synapse_refractory_period: 0,
        signal_propagation_length: 0,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let variant_table = [const_zero_variant; layout::VARIANT_LUT_LEN];

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons,
        variant_table: &variant_table,
    };

    let res = backend.upload_shard(handle, upload);
    assert!(matches!(res, Err(ComputeApiError::SizeMismatch)));

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_rejects_foreign_invalid_freed_handles() {
    if !is_gpu_available() {
        return;
    }
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let foreign_handle =
        compute_api::VramHandle::from_raw_parts(BackendKind::Cpu, handle.id(), handle.generation());
    let res = backend.free_shard(foreign_handle);
    assert!(matches!(res, Err(ComputeApiError::ForeignHandle)));

    backend.free_shard(handle).unwrap();
    let res_freed = backend.free_shard(handle);
    assert!(matches!(res_freed, Err(ComputeApiError::AlreadyFreed)));

    let stale_handle = compute_api::VramHandle::from_raw_parts(
        BackendKind::Cuda,
        handle.id(),
        handle.generation() + 10,
    );
    let res_stale = backend.free_shard(stale_handle);
    assert!(matches!(res_stale, Err(ComputeApiError::InvalidHandle)));
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_teardown_invalidates_existing_handles() {
    if !is_gpu_available() {
        return;
    }
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    backend.teardown().unwrap();

    let res = backend.free_shard(handle);
    assert!(matches!(res, Err(ComputeApiError::InvalidHandle)));
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_variant_table_upload_smoke() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

    let test_state = vec![0u8; state_size];
    let test_axons = vec![0u8; axons_size];

    let const_variant = layout::VariantParameters {
        threshold: 1000,
        rest_potential: -70,
        leak_shift: 5,
        homeostasis_penalty: 2,
        spontaneous_firing_period_ticks: 10,
        initial_synapse_weight: 4000,
        gsop_potentiation: 10,
        gsop_depression: 5,
        homeostasis_decay: 1,
        refractory_period: 3,
        synapse_refractory_period: 2,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [0, 1, 2, 3, 4, 5, 6, 7],
        ahp_amplitude: 2,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 1,
        adaptive_leak_gain: 2,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let mut variant_table = [const_variant; layout::VARIANT_LUT_LEN];
    variant_table[1].threshold = 2000;
    variant_table[2].rest_potential = -60;

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons,
        variant_table: &variant_table,
    };

    backend.upload_shard(handle, upload).unwrap();

    backend.free_shard(handle).unwrap();
}

#[test]
fn test_cuda_invalid_alloc_spec() {
    let mut backend = CudaBackend {
        _config: CudaBackendConfig::default(),
        registry: ResourceRegistry::default(),
        _marker: std::marker::PhantomData,
    };

    // padded_n = 0 -> InvalidShape
    let spec_zero = compute_api::ShardAllocSpec {
        padded_n: 0,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let res_zero = backend.alloc_shard(spec_zero);
    assert!(matches!(res_zero, Err(ComputeApiError::InvalidShape)));

    // padded_n not aligned to 64 -> AlignmentViolation
    let spec_unaligned = compute_api::ShardAllocSpec {
        padded_n: 63,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let res_unaligned = backend.alloc_shard(spec_unaligned);
    assert!(matches!(
        res_unaligned,
        Err(ComputeApiError::AlignmentViolation)
    ));
}

#[test]
#[cfg(not(feature = "native"))]
fn test_run_day_batch_without_native_returns_unsupported_feature() {
    let mut backend = CudaBackend {
        _config: CudaBackendConfig::default(),
        registry: ResourceRegistry::default(),
        _marker: std::marker::PhantomData,
    };

    let handle = compute_api::VramHandle::from_raw_parts(
        BackendKind::Cuda,
        core::num::NonZeroU64::new(1).unwrap(),
        1,
    );
    let mut output_spikes = [0u32; 1];
    let mut output_spike_counts = [0u32; 1];
    let cmd = compute_api::DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 1,
        num_outputs: 0,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        mapped_soma_ids: &[],
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    let res = backend.run_day_batch(handle, cmd);
    assert!(matches!(res, Err(ComputeApiError::UnsupportedFeature)));
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_propagate_uploaded_axons() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    // 1. Test calling on non-uploaded backend returns BackendNotInitialized
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 3,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let res_not_init = backend.propagate_uploaded_axons_for_test(handle, 5);
    assert!(matches!(
        res_not_init,
        Err(ComputeApiError::BackendNotInitialized)
    ));

    // 2. Upload axons and state
    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

    let test_state = vec![0u8; state_size];

    // Formulate test axons
    // Header: magic "AXAX", version 1, total_axons = 3, _padding = 0
    let header = layout::AxonsFileHeader::new(spec.total_axons);
    let mut test_axons_blob = vec![0u8; axons_size];
    test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

    // Create BurstHeads8 for 3 axons
    let mut heads = [
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
    ];

    // Populate various heads
    // Axon 0
    heads[0].h0 = 100; // normal active
    heads[0].h1 = types::AXON_SENTINEL; // sentinel (inactive)
    heads[0].h2 = types::AXON_SENTINEL - 1; // active, edge case
    heads[0].h3 = types::AXON_SENTINEL + 1; // active, value above sentinel

    // Axon 1
    heads[1].h0 = 10;
    heads[1].h1 = 20;
    heads[1].h2 = types::AXON_SENTINEL;

    // Copy heads into the blob
    let heads_bytes = bytemuck::cast_slice(&heads);
    test_axons_blob[16..16 + heads_bytes.len()].copy_from_slice(heads_bytes);

    let const_zero_variant = layout::VariantParameters {
        threshold: 0,
        rest_potential: 0,
        leak_shift: 0,
        homeostasis_penalty: 0,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 0,
        refractory_period: 0,
        synapse_refractory_period: 0,
        signal_propagation_length: 0,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let variant_table = [const_zero_variant; layout::VARIANT_LUT_LEN];

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    // 3. Test invalid v_seg (0 or > 255) returns InvalidBatch
    let res_invalid_vseg0 = backend.propagate_uploaded_axons_for_test(handle, 0);
    assert!(matches!(
        res_invalid_vseg0,
        Err(ComputeApiError::InvalidBatch)
    ));
    let res_invalid_vseg256 = backend.propagate_uploaded_axons_for_test(handle, 256);
    assert!(matches!(
        res_invalid_vseg256,
        Err(ComputeApiError::InvalidBatch)
    ));

    // 4. Run propagation primitive on GPU
    let v_seg = 5;
    backend
        .propagate_uploaded_axons_for_test(handle, v_seg)
        .unwrap();

    // 5. Read back via debug_snapshot
    let mut snap_state = vec![0u8; state_size];
    let mut snap_axons_blob = vec![0u8; axons_size];
    let snapshot = compute_api::ShardSnapshotMut {
        state_blob: &mut snap_state,
        axons_blob: &mut snap_axons_blob,
    };
    backend.debug_snapshot(handle, snapshot).unwrap();

    // 6. Compare with CPU-expected propagate_head for each head
    let mut expected_heads = heads;

    let update_head = |h: &mut u32| {
        *h = physics::propagate_head(*h, v_seg);
    };

    // Axon 0
    update_head(&mut expected_heads[0].h0);
    update_head(&mut expected_heads[0].h1);
    update_head(&mut expected_heads[0].h2);
    update_head(&mut expected_heads[0].h3);
    update_head(&mut expected_heads[0].h4);
    update_head(&mut expected_heads[0].h5);
    update_head(&mut expected_heads[0].h6);
    update_head(&mut expected_heads[0].h7);

    // Axon 1
    update_head(&mut expected_heads[1].h0);
    update_head(&mut expected_heads[1].h1);
    update_head(&mut expected_heads[1].h2);
    update_head(&mut expected_heads[1].h3);
    update_head(&mut expected_heads[1].h4);
    update_head(&mut expected_heads[1].h5);
    update_head(&mut expected_heads[1].h6);
    update_head(&mut expected_heads[1].h7);

    // Axon 2
    update_head(&mut expected_heads[2].h0);
    update_head(&mut expected_heads[2].h1);
    update_head(&mut expected_heads[2].h2);
    update_head(&mut expected_heads[2].h3);
    update_head(&mut expected_heads[2].h4);
    update_head(&mut expected_heads[2].h5);
    update_head(&mut expected_heads[2].h6);
    update_head(&mut expected_heads[2].h7);

    let mut expected_axons_blob = vec![0u8; axons_size];
    expected_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));
    expected_axons_blob[16..16 + heads_bytes.len()]
        .copy_from_slice(bytemuck::cast_slice(&expected_heads));

    assert_eq!(snap_axons_blob, expected_axons_blob);

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_inject_and_propagate_axons_tick() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    // 1. Create a shard with 4 axons, virtual offset 100
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 4,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    // Check non-uploaded returns BackendNotInitialized
    let res_not_init =
        backend.inject_and_propagate_axons_tick_for_test(handle, 5, 95, 10, None, None);
    assert!(matches!(
        res_not_init,
        Err(ComputeApiError::BackendNotInitialized)
    ));

    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

    // Fill initial axons with sentinels
    let header = layout::AxonsFileHeader::new(spec.total_axons);
    let mut initial_axons_blob = vec![0u8; axons_size];
    initial_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

    let mut heads = [
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
    ];

    // Seed some initial heads to verify propagation and shifting
    heads[0].h0 = 1000;
    heads[0].h1 = types::AXON_SENTINEL;
    heads[1].h0 = 2000;
    heads[2].h0 = 3000;
    heads[3].h0 = 4000;

    let heads_bytes = bytemuck::cast_slice(&heads);
    initial_axons_blob[16..16 + heads_bytes.len()].copy_from_slice(heads_bytes);

    let const_zero_variant = layout::VariantParameters {
        threshold: 0,
        rest_potential: 0,
        leak_shift: 0,
        homeostasis_penalty: 0,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 0,
        refractory_period: 0,
        synapse_refractory_period: 0,
        signal_propagation_length: 0,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let variant_table = [const_zero_variant; layout::VARIANT_LUT_LEN];

    let upload = compute_api::ShardUpload {
        state_blob: &vec![0u8; state_size],
        axons_blob: &initial_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    // Check invalid v_seg range checks
    let res_vseg0 = backend.inject_and_propagate_axons_tick_for_test(handle, 0, 95, 10, None, None);
    assert!(matches!(res_vseg0, Err(ComputeApiError::InvalidBatch)));

    let res_vseg256 =
        backend.inject_and_propagate_axons_tick_for_test(handle, 256, 95, 10, None, None);
    assert!(matches!(res_vseg256, Err(ComputeApiError::InvalidBatch)));

    // Inputs for the tick:
    // Virtual Offset: 95
    // Num Virtual Axons: 10
    // Virtual Axon range: 95..105 (global IDs)
    // bitmask: 1 word, active bits: 0, 5, 6, 7, 9
    // - bit 0 (global ID 95): active, but ignored
    // - bit 5 (global ID 100): active -> axon 0 gets 1 virtual injection
    // - bit 6 (global ID 101): active -> axon 1 gets 1 virtual injection
    // - bit 7 (global ID 102): active -> axon 2 gets 1 virtual injection
    // - bit 9 (global ID 104): active, but ignored
    let input_bitmask = vec![737];

    // Incoming spikes:
    // Spikes list: [1, 2, 2, 9] (local axon IDs)
    // - axon 1 (local ID 1) gets 1 incoming injection
    // - axon 2 (local ID 2) gets 2 incoming injections (duplicate!)
    // - local ID 9 gets ignored
    let incoming_spikes = vec![1, 2, 2, 9];

    // Let's run the tick on GPU
    let v_seg = 5;
    backend
        .inject_and_propagate_axons_tick_for_test(
            handle,
            v_seg,
            95,
            10,
            Some(&input_bitmask),
            Some(&incoming_spikes),
        )
        .unwrap();

    // Read back
    let mut snap_state = vec![0u8; state_size];
    let mut snap_axons = vec![0u8; axons_size];
    backend
        .debug_snapshot(
            handle,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    // Calculate expected on CPU
    let init_val = physics::initial_axon_head(v_seg);
    let mut expected_heads = heads;

    let apply_injections_and_propagate = |heads_arr: &mut layout::BurstHeads8, n: usize| {
        let mut h_slice = [
            heads_arr.h0,
            heads_arr.h1,
            heads_arr.h2,
            heads_arr.h3,
            heads_arr.h4,
            heads_arr.h5,
            heads_arr.h6,
            heads_arr.h7,
        ];
        for _ in 0..n {
            h_slice[7] = h_slice[6];
            h_slice[6] = h_slice[5];
            h_slice[5] = h_slice[4];
            h_slice[4] = h_slice[3];
            h_slice[3] = h_slice[2];
            h_slice[2] = h_slice[1];
            h_slice[1] = h_slice[0];
            h_slice[0] = init_val;
        }
        for h in h_slice.iter_mut() {
            *h = physics::propagate_head(*h, v_seg);
        }
        heads_arr.h0 = h_slice[0];
        heads_arr.h1 = h_slice[1];
        heads_arr.h2 = h_slice[2];
        heads_arr.h3 = h_slice[3];
        heads_arr.h4 = h_slice[4];
        heads_arr.h5 = h_slice[5];
        heads_arr.h6 = h_slice[6];
        heads_arr.h7 = h_slice[7];
    };

    apply_injections_and_propagate(&mut expected_heads[0], 1); // Axon 0
    apply_injections_and_propagate(&mut expected_heads[1], 2); // Axon 1
    apply_injections_and_propagate(&mut expected_heads[2], 3); // Axon 2
    apply_injections_and_propagate(&mut expected_heads[3], 0); // Axon 3 (no injections)

    let mut expected_axons_blob = vec![0u8; axons_size];
    expected_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));
    expected_axons_blob[16..16 + heads_bytes.len()]
        .copy_from_slice(bytemuck::cast_slice(&expected_heads));

    assert_eq!(snap_axons, expected_axons_blob);

    // Test no-input case matches simple propagation
    let upload2 = compute_api::ShardUpload {
        state_blob: &vec![0u8; state_size],
        axons_blob: &initial_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload2).unwrap();

    backend
        .inject_and_propagate_axons_tick_for_test(handle, v_seg, 95, 10, None, None)
        .unwrap();

    backend
        .debug_snapshot(
            handle,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    let mut expected_heads2 = heads;
    for eh in expected_heads2.iter_mut() {
        apply_injections_and_propagate(eh, 0);
    }
    let mut expected_axons_blob2 = vec![0u8; axons_size];
    expected_axons_blob2[..16].copy_from_slice(bytemuck::bytes_of(&header));
    expected_axons_blob2[16..16 + heads_bytes.len()]
        .copy_from_slice(bytemuck::cast_slice(&expected_heads2));

    assert_eq!(snap_axons, expected_axons_blob2);

    // 1. Test short input_bitmask fails with InvalidBatch
    let short_mask = vec![];
    let res_short = backend.inject_and_propagate_axons_tick_for_test(
        handle,
        v_seg,
        95,
        10,
        Some(&short_mask),
        None,
    );
    assert!(matches!(res_short, Err(ComputeApiError::InvalidBatch)));

    // 2. Test virtual range near u32::MAX does not overflow
    let mask_ok = vec![0; 1];
    let res_u32_max = backend.inject_and_propagate_axons_tick_for_test(
        handle,
        v_seg,
        u32::MAX - 5,
        10,
        Some(&mask_ok),
        None,
    );
    assert!(res_u32_max.is_ok());

    // 3. Test 9+ duplicate incoming spikes in one axon
    let upload3 = compute_api::ShardUpload {
        state_blob: &vec![0u8; state_size],
        axons_blob: &initial_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload3).unwrap();

    let duplicate_spikes = vec![1; 10]; // 10 spikes for local axon 1
    backend
        .inject_and_propagate_axons_tick_for_test(
            handle,
            v_seg,
            95,
            10,
            None,
            Some(&duplicate_spikes),
        )
        .unwrap();

    backend
        .debug_snapshot(
            handle,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    let mut expected_heads3 = heads;
    apply_injections_and_propagate(&mut expected_heads3[0], 0);
    apply_injections_and_propagate(&mut expected_heads3[1], 8); // Clamped to 8
    apply_injections_and_propagate(&mut expected_heads3[2], 0);
    apply_injections_and_propagate(&mut expected_heads3[3], 0);

    let mut expected_axons_blob3 = vec![0u8; axons_size];
    expected_axons_blob3[..16].copy_from_slice(bytemuck::bytes_of(&header));
    expected_axons_blob3[16..16 + heads_bytes.len()]
        .copy_from_slice(bytemuck::cast_slice(&expected_heads3));

    assert_eq!(snap_axons, expected_axons_blob3);

    // 4. Test virtual range near u32::MAX with actual injections (overflow check)
    let spec_overflow = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: u32::MAX - 1,
    };
    let handle_overflow = backend.alloc_shard(spec_overflow).unwrap();
    let state_size_ov = layout::calculate_state_blob_size(spec_overflow.padded_n as usize);
    let axons_size_ov =
        compute_api::validation::expected_axons_blob_size(spec_overflow.total_axons).unwrap();

    let header_ov = layout::AxonsFileHeader::new(spec_overflow.total_axons);
    let mut initial_axons_blob_ov = vec![0u8; axons_size_ov];
    initial_axons_blob_ov[..16].copy_from_slice(bytemuck::bytes_of(&header_ov));

    let mut heads_ov = [
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
    ];
    heads_ov[0].h0 = 1000;
    heads_ov[1].h0 = 2000;

    let heads_bytes_ov = bytemuck::cast_slice(&heads_ov);
    initial_axons_blob_ov[16..16 + heads_bytes_ov.len()].copy_from_slice(heads_bytes_ov);

    let upload_ov = compute_api::ShardUpload {
        state_blob: &vec![0u8; state_size_ov],
        axons_blob: &initial_axons_blob_ov,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle_overflow, upload_ov).unwrap();

    // cmd_virtual_offset = u32::MAX - 1, num_virtual_axons = 2, bitmask = [3] (activates both)
    let mask_overflow = vec![3];
    backend
        .inject_and_propagate_axons_tick_for_test(
            handle_overflow,
            v_seg,
            u32::MAX - 1,
            2,
            Some(&mask_overflow),
            None,
        )
        .unwrap();

    let mut snap_state_ov = vec![0u8; state_size_ov];
    let mut snap_axons_ov = vec![0u8; axons_size_ov];
    backend
        .debug_snapshot(
            handle_overflow,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state_ov,
                axons_blob: &mut snap_axons_ov,
            },
        )
        .unwrap();

    // Both axons should have 1 injection (virtual) and then propagated
    let mut expected_heads_ov = heads_ov;
    apply_injections_and_propagate(&mut expected_heads_ov[0], 1);
    apply_injections_and_propagate(&mut expected_heads_ov[1], 1);

    let mut expected_axons_blob_ov = vec![0u8; axons_size_ov];
    expected_axons_blob_ov[..16].copy_from_slice(bytemuck::bytes_of(&header_ov));
    expected_axons_blob_ov[16..16 + heads_bytes_ov.len()]
        .copy_from_slice(bytemuck::cast_slice(&expected_heads_ov));

    assert_eq!(snap_axons_ov, expected_axons_blob_ov);
    backend.free_shard(handle_overflow).unwrap();

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
#[allow(clippy::needless_range_loop, clippy::identity_op, clippy::erasing_op)]
fn test_cuda_native_compute_input_current_probe() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 3,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

    // 1. Set up state blob
    let mut test_state = vec![0u8; state_size];
    let offsets = layout::compute_state_offsets(spec.padded_n as usize);

    let mut targets =
        vec![types::PackedTarget::NONE; layout::MAX_DENDRITES * spec.padded_n as usize];
    let mut weights = vec![0i32; layout::MAX_DENDRITES * spec.padded_n as usize];

    // Soma 0
    // Dendrite 0: target axon 1, segment 10. weight = 1600 << 16
    targets[0 * spec.padded_n as usize + 0] = types::PackedTarget::pack(1, 10);
    weights[0 * spec.padded_n as usize + 0] = 1600 << 16;
    // Dendrite 1: target = NONE, weight = 500 << 16
    targets[1 * spec.padded_n as usize + 0] = types::PackedTarget::NONE;
    weights[1 * spec.padded_n as usize + 0] = 500 << 16;
    // Dendrite 2: target axon_q = 0x00FF_FFFF (corrupt/reserved), weight = 999 << 16 (ignored)
    targets[2 * spec.padded_n as usize + 0] = types::PackedTarget(0x0AFF_FFFF);
    weights[2 * spec.padded_n as usize + 0] = 999 << 16;
    // Dendrite 3: target axon 0, segment 0. weight = -800 << 16
    targets[3 * spec.padded_n as usize + 0] = types::PackedTarget::pack(0, 0);
    weights[3 * spec.padded_n as usize + 0] = -800 << 16;

    // Soma 1
    // Dendrite 0: target = TOMBSTONE (inactive)
    targets[0 * spec.padded_n as usize + 1] = types::PackedTarget::TOMBSTONE;
    weights[0 * spec.padded_n as usize + 1] = 500 << 16;
    // Dendrite 1: target axon 2, segment 50 (miss). weight = 3200 << 16
    targets[1 * spec.padded_n as usize + 1] = types::PackedTarget::pack(2, 50);
    weights[1 * spec.padded_n as usize + 1] = 3200 << 16;

    // Soma 2
    // Dendrite 0: target axon 1, segment 0 (miss). weight = i32::MAX
    targets[0 * spec.padded_n as usize + 2] = types::PackedTarget::pack(1, 0);
    weights[0 * spec.padded_n as usize + 2] = i32::MAX;
    // Dendrite 1: target axon 1, segment 0 (miss). weight = i32::MAX
    targets[1 * spec.padded_n as usize + 2] = types::PackedTarget::pack(1, 0);
    weights[1 * spec.padded_n as usize + 2] = i32::MAX;
    // Dendrite 2: target axon 1, segment 10 (hit). weight = i32::MIN
    targets[2 * spec.padded_n as usize + 2] = types::PackedTarget::pack(1, 10);
    weights[2 * spec.padded_n as usize + 2] = i32::MIN;
    // Dendrite 3: target axon 1, segment 10 (hit). weight = i32::MIN
    targets[3 * spec.padded_n as usize + 2] = types::PackedTarget::pack(1, 10);
    weights[3 * spec.padded_n as usize + 2] = i32::MIN;

    // Copy targets and weights to state blob
    let targets_bytes = bytemuck::cast_slice(&targets);
    test_state[offsets.off_targets..offsets.off_targets + targets_bytes.len()]
        .copy_from_slice(targets_bytes);

    let weights_bytes = bytemuck::cast_slice(&weights);
    test_state[offsets.off_weights..offsets.off_weights + weights_bytes.len()]
        .copy_from_slice(weights_bytes);

    // 2. Set up axon heads
    let header = layout::AxonsFileHeader::new(spec.total_axons);
    let mut test_axons_blob = vec![0u8; axons_size];
    test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

    let mut heads = [
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
    ];
    // Axon 0
    heads[0].h0 = 0; // segment 0 (active)
                     // Axon 1
    heads[1].h0 = 10; // segment 10 (active)
                      // Axon 2
    heads[2].h0 = 20; // segment 20 (active)

    let heads_bytes = bytemuck::cast_slice(&heads);
    test_axons_blob[16..16 + heads_bytes.len()].copy_from_slice(heads_bytes);

    let const_zero_variant = layout::VariantParameters {
        threshold: 0,
        rest_potential: 0,
        leak_shift: 0,
        homeostasis_penalty: 0,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 0,
        refractory_period: 0,
        synapse_refractory_period: 0,
        signal_propagation_length: 5, // prop len = 5
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let variant_table = [const_zero_variant; layout::VARIANT_LUT_LEN];

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    // 3. Test compute input current probe
    // Output buffer size is larger than padded_n to test that the tail is not overwritten
    let mut out_i_in = vec![0i32; spec.padded_n as usize + 10];
    for i in spec.padded_n as usize..spec.padded_n as usize + 10 {
        out_i_in[i] = 999;
    }

    backend
        .compute_input_current_probe_for_test(handle, &mut out_i_in)
        .unwrap();

    // Soma 0: Dendrite 0 (hit, weight = 1600 >> 16 -> charge 1600)
    //         Dendrite 3 (hit, weight = -800 >> 16 -> charge -800)
    //         Expected = 800
    assert_eq!(out_i_in[0], 800);

    // Soma 1: Dendrite 1 (miss, segment 50 vs head 20 -> d = 20 - 50 = 0xFFFFFFE2 >= 5 -> miss)
    //         Expected = 0
    assert_eq!(out_i_in[1], 0);

    // Soma 2: Dendrite 2 (hit, weight = i32::MIN -> charge = -32768)
    //         Dendrite 3 (hit, weight = i32::MIN -> charge = -32768)
    //         Expected = -65536 (computed via i32::wrapping_add on GPU)
    let expected_soma2 = (i32::MIN >> 16).wrapping_add(i32::MIN >> 16);
    assert_eq!(out_i_in[2], expected_soma2);

    // Remaining somas (3..64): expected = 0
    for i in 3..spec.padded_n as usize {
        assert_eq!(out_i_in[i], 0, "Soma {} mismatch", i);
    }

    // Check that the tail (from padded_n onwards) remains untouched
    for i in spec.padded_n as usize..spec.padded_n as usize + 10 {
        assert_eq!(out_i_in[i], 999, "Tail element {} was overwritten", i);
    }

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
#[allow(clippy::bool_assert_comparison)]
fn test_cuda_native_apply_glif_membrane_probe() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 1,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::validation::expected_axons_blob_size(spec.total_axons).unwrap();

    // 1. Set up state blob
    let mut test_state = vec![0u8; state_size];
    let offsets = layout::compute_state_offsets(spec.padded_n as usize);

    let mut voltages = vec![0i32; spec.padded_n as usize];
    let mut flags = vec![0u8; spec.padded_n as usize];
    let mut thresh_offsets = vec![0i32; spec.padded_n as usize];
    let mut timers = vec![0u8; spec.padded_n as usize];

    // Variant parameters:
    // Variant 0: normal parameters, leak_shift = 4
    // Variant 1: adaptive leak parameters, leak_shift = 8
    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };

    let mut variant_1 = variant_0;
    variant_1.homeostasis_decay = 256;
    variant_1.leak_shift = 16;
    variant_1.adaptive_leak_min_shift = 2;
    variant_1.adaptive_leak_gain = 4;
    variant_1.adaptive_mode = 1;

    let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
    variant_table[1] = variant_1;

    // Soma 0: normal no-spike voltage update (Variant 0)
    // Starting voltage: -60_000, type = 0
    voltages[0] = -60_000;
    flags[0] = types::SomaFlags::new(false, 0, 0).0;
    thresh_offsets[0] = 0;
    timers[0] = 0;

    // Soma 1: GLIF spike (Variant 0)
    // Starting voltage: -49_000, type = 0
    voltages[1] = -49_000;
    flags[1] = types::SomaFlags::new(false, 0, 0).0;
    thresh_offsets[1] = 0;
    timers[1] = 0;

    // Soma 2: refractory timer (Variant 0)
    // Starting voltage: -80_000, type = 0
    voltages[2] = -80_000;
    flags[2] = types::SomaFlags::new(false, 0, 0).0;
    thresh_offsets[2] = 1000;
    timers[2] = 3;

    // Soma 3: burst_count already 7 (Variant 0)
    // Starting voltage: -49_000, type = 0, burst = 7
    voltages[3] = -49_000;
    flags[3] = types::SomaFlags::new(false, 7, 0).0;
    thresh_offsets[3] = 0;
    timers[3] = 0;

    // Soma 4: adaptive leak (Variant 1)
    // Starting voltage: -60_000, type = 1 (Variant 1)
    voltages[4] = -60_000;
    flags[4] = types::SomaFlags::new(false, 0, 1).0;
    thresh_offsets[4] = 512;
    timers[4] = 0;

    // Write to state blob
    test_state[offsets.off_voltage..offsets.off_voltage + voltages.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + flags.len()].copy_from_slice(&flags);
    test_state[offsets.off_thresh..offsets.off_thresh + thresh_offsets.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&thresh_offsets));
    test_state[offsets.off_timers..offsets.off_timers + timers.len()].copy_from_slice(&timers);

    let header = layout::AxonsFileHeader::new(spec.total_axons);
    let mut test_axons_blob = vec![0u8; axons_size];
    test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    // Input currents
    let mut i_in = vec![0i32; spec.padded_n as usize];
    i_in[0] = 2000;
    i_in[1] = 1000;
    i_in[2] = 5000; // ignored
    i_in[3] = 1000;
    i_in[4] = 2000;

    // Verify that short i_in returns InvalidBatch
    let res_short = backend.apply_glif_membrane_probe_for_test(handle, &i_in[..5]);
    assert!(matches!(res_short, Err(ComputeApiError::InvalidBatch)));

    // Run membrane probe update
    backend
        .apply_glif_membrane_probe_for_test(handle, &i_in)
        .unwrap();

    // Download snapshot
    let mut snap_state = vec![0u8; state_size];
    let mut snap_axons = vec![0u8; axons_size];
    backend
        .debug_snapshot(
            handle,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    let snap_voltages: &[i32] = bytemuck::cast_slice(
        &snap_state[offsets.off_voltage..offsets.off_voltage + spec.padded_n as usize * 4],
    );
    let snap_flags: &[u8] =
        &snap_state[offsets.off_flags..offsets.off_flags + spec.padded_n as usize];
    let snap_thresh: &[i32] = bytemuck::cast_slice(
        &snap_state[offsets.off_thresh..offsets.off_thresh + spec.padded_n as usize * 4],
    );
    let snap_timers: &[u8] =
        &snap_state[offsets.off_timers..offsets.off_timers + spec.padded_n as usize];

    let mut expected_voltages = voltages.clone();
    let mut expected_flags = flags.clone();
    let mut expected_thresh = thresh_offsets.clone();
    let mut expected_timers = timers.clone();

    for i in 0..5 {
        let variant_idx = types::SomaFlags(flags[i]).type_id() as usize;
        let var = variant_table[variant_idx];

        let timer = timers[i];
        let thresh_offset = thresh_offsets[i];
        let voltage = voltages[i];
        let i_in_val = i_in[i];

        let decayed_offset =
            physics::homeostasis_decay(thresh_offset, var.homeostasis_decay as i32);

        if timer > 0 {
            expected_timers[i] = timer - 1;
            expected_thresh[i] = decayed_offset;
            expected_voltages[i] = voltage;
            expected_flags[i] = types::SomaFlags::new(
                false,
                types::SomaFlags(flags[i]).burst_count(),
                variant_idx as u8,
            )
            .0;
        } else {
            let v_new = physics::update_glif_voltage(
                voltage,
                i_in_val,
                var.rest_potential,
                decayed_offset,
                var.leak_shift as i32,
                var.adaptive_leak_gain as i32,
                var.adaptive_leak_min_shift as i32,
                var.adaptive_mode as i32,
            );

            let is_glif = physics::is_glif_spike(v_new, var.threshold, decayed_offset);

            if is_glif {
                expected_voltages[i] = var.rest_potential.wrapping_sub(var.ahp_amplitude as i32);
                expected_timers[i] = var.refractory_period;
                expected_thresh[i] = decayed_offset.wrapping_add(var.homeostasis_penalty);
                let new_burst = (types::SomaFlags(flags[i]).burst_count() + 1).min(7);
                expected_flags[i] = types::SomaFlags::new(true, new_burst, variant_idx as u8).0;
            } else {
                expected_voltages[i] = v_new;
                expected_timers[i] = 0;
                expected_thresh[i] = decayed_offset;
                expected_flags[i] = types::SomaFlags::new(
                    false,
                    types::SomaFlags(flags[i]).burst_count(),
                    variant_idx as u8,
                )
                .0;
            }
        }
    }

    // Hardcoded assertions derived from CPU expected values
    assert_eq!(expected_voltages[0], -58625);
    assert_eq!(expected_voltages[1], -80000);
    assert_eq!(expected_voltages[2], -80000);
    assert_eq!(expected_voltages[3], -80000);
    assert_eq!(expected_voltages[4], -58002);

    for i in 0..5 {
        assert_eq!(
            snap_voltages[i], expected_voltages[i],
            "Soma {} voltage mismatch",
            i
        );
        assert_eq!(
            snap_flags[i], expected_flags[i],
            "Soma {} flags mismatch",
            i
        );
        assert_eq!(
            snap_thresh[i], expected_thresh[i],
            "Soma {} thresh mismatch",
            i
        );
        assert_eq!(
            snap_timers[i], expected_timers[i],
            "Soma {} timer mismatch",
            i
        );
    }

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_multi_shard_variant_table_isolation() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec_a = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 1,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle_a = backend.alloc_shard(spec_a).unwrap();

    let spec_b = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 1,
        total_ghosts: 0,
        virtual_offset: 200,
    };
    let handle_b = backend.alloc_shard(spec_b).unwrap();

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(1).unwrap();

    // 1. Shard A upload: variant A (leak_shift = 4)
    let mut test_state_a = vec![0u8; state_size];
    let offsets = layout::compute_state_offsets(64);
    let mut voltages_a = vec![0i32; 64];
    voltages_a[0] = -60_000;
    test_state_a[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages_a));

    let variant_a = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4, // leak_shift = 4
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let variant_table_a = [variant_a; layout::VARIANT_LUT_LEN];

    let header = layout::AxonsFileHeader::new(1);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));

    backend
        .upload_shard(
            handle_a,
            compute_api::ShardUpload {
                state_blob: &test_state_a,
                axons_blob: &test_axons,
                variant_table: &variant_table_a,
            },
        )
        .unwrap();

    // 2. Shard B upload: variant B (leak_shift = 8)
    let test_state_b = vec![0u8; state_size];
    let mut variant_b = variant_a;
    variant_b.leak_shift = 8; // leak_shift = 8
    let variant_table_b = [variant_b; layout::VARIANT_LUT_LEN];

    backend
        .upload_shard(
            handle_b,
            compute_api::ShardUpload {
                state_blob: &test_state_b,
                axons_blob: &test_axons,
                variant_table: &variant_table_b,
            },
        )
        .unwrap();

    // 3. Call run_current_glif_tick_probe_for_test on Shard A
    backend
        .run_current_glif_tick_probe_for_test(handle_a)
        .unwrap();

    // 4. Download Shard A snapshot and check it matches variant A (leak_shift = 4)
    let mut snap_state_a = vec![0u8; state_size];
    let mut snap_axons_a = vec![0u8; axons_size];
    backend
        .debug_snapshot(
            handle_a,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state_a,
                axons_blob: &mut snap_axons_a,
            },
        )
        .unwrap();

    let snap_voltages_a: &[i32] =
        bytemuck::cast_slice(&snap_state_a[offsets.off_voltage..offsets.off_voltage + 256]);

    // Under variant A (leak_shift = 4): expected = -60625
    assert_eq!(snap_voltages_a[0], -60625);

    backend.free_shard(handle_a).unwrap();
    backend.free_shard(handle_b).unwrap();
}

#[test]
#[cfg(feature = "native")]
#[allow(clippy::needless_range_loop, clippy::identity_op, clippy::erasing_op)]
fn test_cuda_native_current_glif_tick_probe() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 3,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(3).unwrap();

    let mut test_state = vec![0u8; state_size];
    let offsets = layout::compute_state_offsets(64);

    // Synapses targets & weights:
    let mut dendrite_targets = vec![0u32; 64 * 128];
    let mut dendrite_weights = vec![0i32; 64 * 128];

    // Soma 0: target local axon 0 segment 5, positive weight (charge 1600)
    dendrite_targets[0 * 64 + 0] = types::PackedTarget::pack(0, 5).0;
    dendrite_weights[0 * 64 + 0] = 1600 << 16;

    // Soma 1: target local axon 1 segment 10, negative weight (charge -800)
    dendrite_targets[0 * 64 + 1] = types::PackedTarget::pack(1, 10).0;
    dendrite_weights[0 * 64 + 1] = -800 << 16;

    // Soma 2: target local axon 2 segment 20, positive weight (charge 500)
    dendrite_targets[0 * 64 + 2] = types::PackedTarget::pack(2, 20).0;
    dendrite_weights[0 * 64 + 2] = 500 << 16;

    // Copy dendrite planes to state blob
    test_state[offsets.off_targets..offsets.off_targets + dendrite_targets.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + dendrite_weights.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    // Initialize soma planes
    let mut voltages = vec![0i32; 64];
    let mut flags = vec![0u8; 64];
    let mut thresh_offsets = vec![0i32; 64];
    let mut timers = vec![0u8; 64];

    // Soma 0: normal voltage trigger spike
    voltages[0] = -49_000;
    flags[0] = types::SomaFlags::new(false, 0, 0).0;
    thresh_offsets[0] = 0;
    timers[0] = 0;

    // Soma 1: normal negative update, no spike
    voltages[1] = -60_000;
    flags[1] = types::SomaFlags::new(false, 0, 0).0;
    thresh_offsets[1] = 0;
    timers[1] = 0;

    // Soma 2: refractory period, decrement timer, decay threshold
    voltages[2] = -80_000;
    flags[2] = types::SomaFlags::new(false, 0, 0).0;
    thresh_offsets[2] = 1000;
    timers[2] = 3;

    // Soma 3: check variant 2 and type_id preservation
    voltages[3] = -60_000;
    flags[3] = types::SomaFlags::new(false, 0, 2).0;
    thresh_offsets[3] = 0;
    timers[3] = 0;

    // Copy soma planes to state blob
    test_state[offsets.off_voltage..offsets.off_voltage + voltages.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + flags.len()].copy_from_slice(&flags);
    test_state[offsets.off_thresh..offsets.off_thresh + thresh_offsets.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&thresh_offsets));
    test_state[offsets.off_timers..offsets.off_timers + timers.len()].copy_from_slice(&timers);

    // Axon heads Setup:
    let header = layout::AxonsFileHeader::new(3);
    let mut test_axons_blob = vec![0u8; axons_size];
    test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

    let mut heads = [
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
        layout::BurstHeads8::empty(types::AXON_SENTINEL),
    ];
    heads[0].h0 = 5;
    heads[1].h0 = 10;
    heads[2].h0 = 20;
    test_axons_blob[16..16 + 3 * 32].copy_from_slice(bytemuck::cast_slice(&heads));

    // Variant parameters:
    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };

    let mut variant_2 = variant_0;
    variant_2.rest_potential = -65_000;
    variant_2.leak_shift = 5;

    let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
    variant_table[2] = variant_2;

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    // 3. Run tick probe
    backend
        .run_current_glif_tick_probe_for_test(handle)
        .unwrap();

    // 4. Download snapshot
    let mut snap_state = vec![0u8; state_size];
    let mut snap_axons = vec![0u8; axons_size];
    backend
        .debug_snapshot(
            handle,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    // Check that axons snapshot is byte-exact compared to original axons blob
    assert_eq!(snap_axons, test_axons_blob);

    let snap_voltages: &[i32] =
        bytemuck::cast_slice(&snap_state[offsets.off_voltage..offsets.off_voltage + 256]);
    let snap_flags: &[u8] = &snap_state[offsets.off_flags..offsets.off_flags + 64];
    let snap_thresh: &[i32] =
        bytemuck::cast_slice(&snap_state[offsets.off_thresh..offsets.off_thresh + 256]);
    let snap_timers: &[u8] = &snap_state[offsets.off_timers..offsets.off_timers + 64];

    // Compute expectations on CPU
    let mut expected_voltages = voltages.clone();
    let mut expected_flags = flags.clone();
    let mut expected_thresh = thresh_offsets.clone();
    let mut expected_timers = timers.clone();
    let mut expected_i_in = vec![0i32; 64];

    for s in 0..64 {
        let variant_idx = types::SomaFlags(flags[s]).type_id() as usize;
        let var = variant_table[variant_idx];
        let propagation_length = var.signal_propagation_length as u32;

        let mut charge_sum = 0i32;
        for d in 0..128 {
            let d_idx = d * 64 + s;
            let target_raw = dendrite_targets[d_idx];
            if target_raw != 0 {
                let target = types::PackedTarget(target_raw);
                if let Some((axon_id, segment_index)) = target.unpack() {
                    let local_axon = axon_id as usize;
                    if local_axon < spec.total_axons as usize {
                        let mut axon_heads_array = [types::AXON_SENTINEL; 8];
                        let h = heads[local_axon];
                        axon_heads_array[0] = h.h0;
                        axon_heads_array[1] = h.h1;
                        axon_heads_array[2] = h.h2;
                        axon_heads_array[3] = h.h3;
                        axon_heads_array[4] = h.h4;
                        axon_heads_array[5] = h.h5;
                        axon_heads_array[6] = h.h6;
                        axon_heads_array[7] = h.h7;

                        if physics::active_tail_hit(
                            &axon_heads_array,
                            segment_index,
                            propagation_length,
                        ) {
                            let weight = dendrite_weights[d_idx];
                            charge_sum = charge_sum.wrapping_add(physics::weight_to_charge(weight));
                        }
                    }
                }
            }
        }
        expected_i_in[s] = charge_sum;
    }

    for i in 0..4 {
        let variant_idx = types::SomaFlags(flags[i]).type_id() as usize;
        let var = variant_table[variant_idx];

        let timer = timers[i];
        let thresh_offset = thresh_offsets[i];
        let voltage = voltages[i];
        let i_in_val = expected_i_in[i];

        let decayed_offset =
            physics::homeostasis_decay(thresh_offset, var.homeostasis_decay as i32);

        if timer > 0 {
            expected_timers[i] = timer - 1;
            expected_thresh[i] = decayed_offset;
            expected_voltages[i] = voltage;
            expected_flags[i] = types::SomaFlags::new(
                false,
                types::SomaFlags(flags[i]).burst_count(),
                variant_idx as u8,
            )
            .0;
        } else {
            let v_new = physics::update_glif_voltage(
                voltage,
                i_in_val,
                var.rest_potential,
                decayed_offset,
                var.leak_shift as i32,
                var.adaptive_leak_gain as i32,
                var.adaptive_leak_min_shift as i32,
                var.adaptive_mode as i32,
            );

            let is_glif = physics::is_glif_spike(v_new, var.threshold, decayed_offset);

            if is_glif {
                expected_voltages[i] = var.rest_potential.wrapping_sub(var.ahp_amplitude as i32);
                expected_timers[i] = var.refractory_period;
                expected_thresh[i] = decayed_offset.wrapping_add(var.homeostasis_penalty);
                let new_burst = (types::SomaFlags(flags[i]).burst_count() + 1).min(7);
                expected_flags[i] = types::SomaFlags::new(true, new_burst, variant_idx as u8).0;
            } else {
                expected_voltages[i] = v_new;
                expected_timers[i] = 0;
                expected_thresh[i] = decayed_offset;
                expected_flags[i] = types::SomaFlags::new(
                    false,
                    types::SomaFlags(flags[i]).burst_count(),
                    variant_idx as u8,
                )
                .0;
            }
        }
    }

    // Assert expected properties
    assert_eq!(expected_voltages[0], -80_000);
    assert_eq!(expected_voltages[1], -61_425);
    assert_eq!(expected_voltages[2], -80_000);
    assert_eq!(expected_timers[2], 2);
    assert_eq!(expected_thresh[2], 900);
    assert_eq!(types::SomaFlags(expected_flags[3]).type_id(), 2);

    for i in 0..4 {
        assert_eq!(
            snap_voltages[i], expected_voltages[i],
            "Soma {} voltage mismatch",
            i
        );
        assert_eq!(
            snap_flags[i], expected_flags[i],
            "Soma {} flags mismatch",
            i
        );
        assert_eq!(
            snap_thresh[i], expected_thresh[i],
            "Soma {} thresh mismatch",
            i
        );
        assert_eq!(
            snap_timers[i], expected_timers[i],
            "Soma {} timer mismatch",
            i
        );
    }

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_current_glif_final_spike_probe() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 5,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(5).unwrap();

    let mut test_state = vec![0u8; state_size];
    let offsets = layout::compute_state_offsets(64);

    // Initialize soma planes
    let mut voltages = vec![-70_000i32; 64];
    let mut flags = vec![0u8; 64];
    let mut thresh_offsets = vec![0i32; 64];
    let mut timers = vec![0u8; 64];
    let mut soma_to_axon = vec![0xFFFFFFFFu32; 64];

    // Variant parameters for Soma
    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };

    // Variant 1: heartbeat_m = MAX_HEARTBEAT_M
    let mut variant_1 = variant_0;
    variant_1.heartbeat_m = physics::constants::MAX_HEARTBEAT_M;

    let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
    variant_table[1] = variant_1;

    // Soma 0: GLIF-only spike (voltage trigger)
    voltages[0] = -48_000;
    flags[0] = types::SomaFlags::new(false, 0, 0).0; // Type 0 (no heartbeat)
    thresh_offsets[0] = 0;
    timers[0] = 0;
    soma_to_axon[0] = 0;

    // Soma 1: Heartbeat-only spike (no GLIF spike because voltage is low)
    voltages[1] = -60_000;
    flags[1] = types::SomaFlags::new(false, 0, 1).0; // Type 1 (heartbeat always)
    thresh_offsets[1] = 0;
    timers[1] = 0;
    soma_to_axon[1] = 1;

    // Soma 2: GLIF + Heartbeat spike simultaneously
    voltages[2] = -48_000;
    flags[2] = types::SomaFlags::new(false, 0, 1).0; // Type 1 (heartbeat always)
    thresh_offsets[2] = 0;
    timers[2] = 0;
    soma_to_axon[2] = 2;

    // Soma 3: Refractory + Heartbeat spike (timer > 0, decrement timer, heartbeat still fires)
    voltages[3] = -80_000;
    flags[3] = types::SomaFlags::new(false, 0, 1).0; // Type 1 (heartbeat always)
    thresh_offsets[3] = 1000;
    timers[3] = 3;
    soma_to_axon[3] = 3;

    // Soma 4: No spike (voltage low, no heartbeat)
    voltages[4] = -80_000;
    flags[4] = types::SomaFlags::new(false, 0, 0).0;
    thresh_offsets[4] = 0;
    timers[4] = 0;
    soma_to_axon[4] = 4;

    // Copy soma planes to state blob
    test_state[offsets.off_voltage..offsets.off_voltage + voltages.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + flags.len()].copy_from_slice(&flags);
    test_state[offsets.off_thresh..offsets.off_thresh + thresh_offsets.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&thresh_offsets));
    test_state[offsets.off_timers..offsets.off_timers + timers.len()].copy_from_slice(&timers);
    test_state[offsets.off_s2a..offsets.off_s2a + soma_to_axon.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));

    // Axon heads setup
    let header = layout::AxonsFileHeader::new(5);
    let mut test_axons_blob = vec![0u8; axons_size];
    test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

    let heads = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 5];
    test_axons_blob[16..16 + 5 * 32].copy_from_slice(bytemuck::cast_slice(&heads));

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    // Outputs setup
    let mapped_soma_ids = vec![0, 1, 2, 3];
    let max_spikes_per_tick = 3;
    let mut output_spikes = vec![0u32; 3];
    let mut output_spike_counts = vec![0u32; 1];

    // Run final tick probe
    let result = backend
        .run_current_glif_final_tick_probe_for_test(
            handle,
            1, // current_tick
            2, // v_seg
            &mapped_soma_ids,
            max_spikes_per_tick,
            &mut output_spikes,
            &mut output_spike_counts,
        )
        .unwrap();

    // 1. Assert BatchResult
    assert_eq!(result.ticks_executed, 1);
    assert_eq!(result.generated_spikes_count, 4);
    assert_eq!(result.output_spikes_written, 3);
    assert_eq!(result.dropped_spikes_count, 1);
    assert_eq!(output_spike_counts[0], 3);

    // 2. Output spikes check: should be soma 0, 1, 2 (soma 3 dropped)
    assert_eq!(output_spikes[0], 0);
    assert_eq!(output_spikes[1], 1);
    assert_eq!(output_spikes[2], 2);

    // 3. Download snapshot
    let mut snap_state = vec![0u8; state_size];
    let mut snap_axons = vec![0u8; axons_size];
    backend
        .debug_snapshot(
            handle,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    let snap_voltages: &[i32] =
        bytemuck::cast_slice(&snap_state[offsets.off_voltage..offsets.off_voltage + 256]);
    let snap_flags: &[u8] = &snap_state[offsets.off_flags..offsets.off_flags + 64];
    let snap_thresh: &[i32] =
        bytemuck::cast_slice(&snap_state[offsets.off_thresh..offsets.off_thresh + 256]);
    let snap_timers: &[u8] = &snap_state[offsets.off_timers..offsets.off_timers + 64];

    // Soma 0 check (GLIF-only spike):
    // Expected voltage reset: rest_potential - ahp_amplitude = -70,000 - 10,000 = -80,000
    assert_eq!(snap_voltages[0], -80_000);
    // Expected timer set to refractory_period: 5
    assert_eq!(snap_timers[0], 5);
    // Expected threshold offset: decayed_offset + homeostasis_penalty = 0 + 1000 = 1000
    assert_eq!(snap_thresh[0], 1000);
    // Expected flags: spiking bit set, burst_count = 1, type = 0
    let flags_0 = types::SomaFlags(snap_flags[0]);
    assert!(flags_0.spiking());
    assert_eq!(flags_0.burst_count(), 1);
    assert_eq!(flags_0.type_id(), 0);

    // Soma 1 check (Heartbeat-only spike):
    // Expected voltage: no GLIF reset, but leak decay happens (-60,625)
    assert_eq!(snap_voltages[1], -60_625);
    // Expected timer: 0
    assert_eq!(snap_timers[1], 0);
    // Expected threshold: decayed_offset = 0
    assert_eq!(snap_thresh[1], 0);
    // Expected flags: spiking bit set, burst_count = 1, type = 1
    let flags_1 = types::SomaFlags(snap_flags[1]);
    assert!(flags_1.spiking());
    assert_eq!(flags_1.burst_count(), 1);
    assert_eq!(flags_1.type_id(), 1);

    // Soma 2 check (GLIF + Heartbeat spike):
    // Expected voltage: reset to -80,000
    assert_eq!(snap_voltages[2], -80_000);
    // Expected timer: 5
    assert_eq!(snap_timers[2], 5);
    // Expected threshold: 1000
    assert_eq!(snap_thresh[2], 1000);
    // Expected flags: spiking bit set, burst_count = 1 (burst count increments only once per tick), type = 1
    let flags_2 = types::SomaFlags(snap_flags[2]);
    assert!(flags_2.spiking());
    assert_eq!(flags_2.burst_count(), 1);
    assert_eq!(flags_2.type_id(), 1);

    // Soma 3 check (Refractory + Heartbeat spike):
    // Expected voltage: unchanged (-80,000)
    assert_eq!(snap_voltages[3], -80_000);
    // Expected timer: decremented from 3 to 2
    assert_eq!(snap_timers[3], 2);
    // Expected threshold: decayed from 1000 by 100 to 900
    assert_eq!(snap_thresh[3], 900);
    // Expected flags: spiking bit set, burst_count = 1, type = 1
    let flags_3 = types::SomaFlags(snap_flags[3]);
    assert!(flags_3.spiking());
    assert_eq!(flags_3.burst_count(), 1);
    assert_eq!(flags_3.type_id(), 1);

    // Soma 4 check (No spike):
    // Expected voltage: no GLIF reset, but leak decay happens (-79,375)
    assert_eq!(snap_voltages[4], -79_375);
    // Expected timer: 0
    assert_eq!(snap_timers[4], 0);
    // Expected threshold: 0
    assert_eq!(snap_thresh[4], 0);
    // Expected flags: spiking bit clear, burst_count = 0, type = 0
    let flags_4 = types::SomaFlags(snap_flags[4]);
    assert!(!flags_4.spiking());
    assert_eq!(flags_4.burst_count(), 0);
    assert_eq!(flags_4.type_id(), 0);

    // Axon heads check
    let mut snap_heads = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 5];
    unsafe {
        std::ptr::copy_nonoverlapping(
            snap_axons[16..16 + 5 * 32].as_ptr(),
            snap_heads.as_mut_ptr() as *mut u8,
            5 * 32,
        );
    }
    let expected_head = physics::initial_axon_head(2);

    // Axon 0 pushed
    assert_eq!(snap_heads[0].h0, expected_head);
    // Axon 1 pushed
    assert_eq!(snap_heads[1].h0, expected_head);
    // Axon 2 pushed
    assert_eq!(snap_heads[2].h0, expected_head);
    // Axon 3 pushed
    assert_eq!(snap_heads[3].h0, expected_head);
    // Axon 4 NOT pushed
    assert_eq!(snap_heads[4].h0, types::AXON_SENTINEL);

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_glif_extreme_values_final_tick_probe() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 1,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(1).unwrap();

    let mut test_state = vec![0u8; state_size];
    let offsets = layout::compute_state_offsets(64);

    // Initialize soma planes to inactive rest state by default
    let mut voltages = vec![-70_000i32; 64];
    let mut flags = vec![0u8; 64];
    let mut thresh_offsets = vec![0i32; 64];
    let timers = vec![0u8; 64];
    let mut soma_to_axon = vec![0xFFFFFFFFu32; 64];

    // Setup base variant
    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };

    // Variant 1: homeostasis decay test
    let mut variant_1 = variant_0;
    variant_1.homeostasis_decay = 1000;

    // Variant 2: extreme threshold math
    let mut variant_2 = variant_0;
    variant_2.threshold = i32::MIN + 10;
    variant_2.rest_potential = i32::MAX - 10;

    let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
    variant_table[1] = variant_1;
    variant_table[2] = variant_2;

    // Setup Soma 0 (Extreme difference, leak subtraction wrapping)
    voltages[0] = i32::MAX - 10;
    variant_table[0].rest_potential = i32::MIN + 10;
    variant_table[0].threshold = i32::MAX - 100;

    // Setup Soma 1 (Negative wrapping in homeostasis decay)
    thresh_offsets[1] = i32::MIN + 500;
    flags[1] = types::SomaFlags::new(false, 0, 1).0; // Type 1

    // Setup Soma 2 (Extreme effective threshold with spike reset)
    voltages[2] = i32::MAX - 10;
    thresh_offsets[2] = i32::MIN + 20;
    flags[2] = types::SomaFlags::new(false, 0, 2).0; // Type 2
    soma_to_axon[2] = 0; // mapped to axon 0

    // Upload state
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_thresh..offsets.off_thresh + 256]
        .copy_from_slice(bytemuck::cast_slice(&thresh_offsets));
    test_state[offsets.off_timers..offsets.off_timers + 64].copy_from_slice(&timers);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &vec![0u8; axons_size],
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    // Mapped soma IDs (for output spikes)
    let mapped_soma_ids = vec![2u32]; // only soma 2 mapped to output spikes
    let max_spikes_per_tick = 10;
    let mut output_spikes = vec![0u32; max_spikes_per_tick as usize];
    let mut output_spike_counts = vec![0u32; 1];

    // 1. Calculate EXPECTED values via Rust physics implementation
    // Soma 0 expected:
    let s0_decayed =
        physics::homeostasis_decay(thresh_offsets[0], variant_table[0].homeostasis_decay as i32);
    let s0_v_new = physics::update_glif_voltage(
        voltages[0],
        0,
        variant_table[0].rest_potential,
        s0_decayed,
        variant_table[0].leak_shift as i32,
        0,
        0,
        0,
    );
    let s0_spike = physics::is_glif_spike(s0_v_new, variant_table[0].threshold, s0_decayed);
    assert!(!s0_spike);

    // Soma 1 expected:
    let s1_decayed =
        physics::homeostasis_decay(thresh_offsets[1], variant_table[1].homeostasis_decay as i32);
    let s1_v_new = physics::update_glif_voltage(
        voltages[1],
        0,
        variant_table[1].rest_potential,
        s1_decayed,
        variant_table[1].leak_shift as i32,
        0,
        0,
        0,
    );
    let s1_spike = physics::is_glif_spike(s1_v_new, variant_table[1].threshold, s1_decayed);
    assert!(!s1_spike);

    // Soma 2 expected:
    let s2_decayed =
        physics::homeostasis_decay(thresh_offsets[2], variant_table[2].homeostasis_decay as i32);
    let s2_v_new = physics::update_glif_voltage(
        voltages[2],
        0,
        variant_table[2].rest_potential,
        s2_decayed,
        variant_table[2].leak_shift as i32,
        0,
        0,
        0,
    );
    let s2_spike = physics::is_glif_spike(s2_v_new, variant_table[2].threshold, s2_decayed);
    assert!(s2_spike);

    // Run final tick probe
    let result = backend
        .run_current_glif_final_tick_probe_for_test(
            handle,
            1, // current_tick
            2, // v_seg
            &mapped_soma_ids,
            max_spikes_per_tick,
            &mut output_spikes,
            &mut output_spike_counts,
        )
        .unwrap();

    assert_eq!(result.generated_spikes_count, 1); // Only Soma 2 spikes
    assert_eq!(output_spike_counts[0], 1);
    assert_eq!(output_spikes[0], 2); // Soma 2 is mapped output

    // Download snapshot
    let mut snap_state = vec![0u8; state_size];
    let mut snap_axons = vec![0u8; axons_size];
    backend
        .debug_snapshot(
            handle,
            compute_api::ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();

    let snap_voltages: &[i32] =
        bytemuck::cast_slice(&snap_state[offsets.off_voltage..offsets.off_voltage + 256]);
    let snap_flags: &[u8] = &snap_state[offsets.off_flags..offsets.off_flags + 64];
    let snap_thresh: &[i32] =
        bytemuck::cast_slice(&snap_state[offsets.off_thresh..offsets.off_thresh + 256]);
    let snap_timers: &[u8] = &snap_state[offsets.off_timers..offsets.off_timers + 64];

    // Verify Soma 0 (Positive overflow)
    assert_eq!(snap_voltages[0], s0_v_new);
    assert_eq!(snap_timers[0], 0);
    assert_eq!(snap_thresh[0], s0_decayed);
    let flags_0 = types::SomaFlags(snap_flags[0]);
    assert!(!flags_0.spiking());

    // Verify Soma 1 (Negative wrapping)
    assert_eq!(snap_voltages[1], s1_v_new);
    assert_eq!(snap_timers[1], 0);
    assert_eq!(snap_thresh[1], s1_decayed);
    let flags_1 = types::SomaFlags(snap_flags[1]);
    assert!(!flags_1.spiking());

    // Verify Soma 2 (Spike with extreme values)
    let expected_reset_voltage = (variant_table[2].rest_potential as u32)
        .wrapping_sub(variant_table[2].ahp_amplitude as u32)
        as i32;
    assert_eq!(snap_voltages[2], expected_reset_voltage);
    assert_eq!(snap_timers[2], variant_table[2].refractory_period);
    let expected_penalty_thresh = s2_decayed.wrapping_add(variant_table[2].homeostasis_penalty);
    assert_eq!(snap_thresh[2], expected_penalty_thresh);
    let flags_2 = types::SomaFlags(snap_flags[2]);
    assert!(flags_2.spiking());

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
#[allow(clippy::needless_range_loop, clippy::identity_op, clippy::erasing_op)]
fn test_cuda_native_variant_aware_current_probe() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 1,
        total_ghosts: 0,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(1).unwrap();

    let mut test_state = vec![0u8; state_size];
    let offsets = layout::compute_state_offsets(64);

    // Synapses targets & weights:
    let mut dendrite_targets = vec![0u32; 64 * 128];
    let mut dendrite_weights = vec![0i32; 64 * 128];

    // Soma 0 (type_id = 0, Variant 0: signal_propagation_length = 5)
    // target axon 0 segment 5. weight = 1000 << 16 (charge 1000)
    dendrite_targets[0 * 64 + 0] = types::PackedTarget::pack(0, 5).0;
    dendrite_weights[0 * 64 + 0] = 1000 << 16;

    // Soma 1 (type_id = 1, Variant 1: signal_propagation_length = 3)
    // target axon 0 segment 5. weight = 1000 << 16 (charge 1000)
    dendrite_targets[0 * 64 + 1] = types::PackedTarget::pack(0, 5).0;
    dendrite_weights[0 * 64 + 1] = 1000 << 16;

    // Copy targets/weights to state
    test_state[offsets.off_targets..offsets.off_targets + dendrite_targets.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + dendrite_weights.len() * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    // Initialize soma planes
    let mut flags = vec![0u8; 64];
    flags[0] = types::SomaFlags::new(false, 0, 0).0; // Variant 0
    flags[1] = types::SomaFlags::new(false, 0, 1).0; // Variant 1

    test_state[offsets.off_flags..offsets.off_flags + flags.len()].copy_from_slice(&flags);

    // Axon heads Setup:
    let header = layout::AxonsFileHeader::new(1);
    let mut test_axons_blob = vec![0u8; axons_size];
    test_axons_blob[..16].copy_from_slice(bytemuck::bytes_of(&header));

    let mut heads = [layout::BurstHeads8::empty(types::AXON_SENTINEL)];
    // Set head to 9
    heads[0].h0 = 9;
    test_axons_blob[16..16 + 32].copy_from_slice(bytemuck::cast_slice(&heads));

    // Variant parameters:
    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };

    let mut variant_1 = variant_0;
    variant_1.signal_propagation_length = 3;

    let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
    variant_table[1] = variant_1;

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons_blob,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    // 3. Compute input current probe
    let mut out_i_in = vec![0i32; 64];
    backend
        .compute_input_current_probe_for_test(handle, &mut out_i_in)
        .unwrap();

    // Compute expectations on CPU
    let mut expected_i_in = vec![0i32; 64];
    for s in 0..64 {
        let variant_idx = types::SomaFlags(flags[s]).type_id() as usize;
        let var = variant_table[variant_idx];
        let propagation_length = var.signal_propagation_length as u32;

        let mut charge_sum = 0i32;
        for d in 0..128 {
            let d_idx = d * 64 + s;
            let target_raw = dendrite_targets[d_idx];
            if target_raw != 0 {
                let target = types::PackedTarget(target_raw);
                if let Some((axon_id, segment_index)) = target.unpack() {
                    let local_axon = axon_id as usize;
                    if local_axon < spec.total_axons as usize {
                        let mut axon_heads_array = [types::AXON_SENTINEL; 8];
                        let h = heads[local_axon];
                        axon_heads_array[0] = h.h0;
                        axon_heads_array[1] = h.h1;
                        axon_heads_array[2] = h.h2;
                        axon_heads_array[3] = h.h3;
                        axon_heads_array[4] = h.h4;
                        axon_heads_array[5] = h.h5;
                        axon_heads_array[6] = h.h6;
                        axon_heads_array[7] = h.h7;

                        if physics::active_tail_hit(
                            &axon_heads_array,
                            segment_index,
                            propagation_length,
                        ) {
                            let weight = dendrite_weights[d_idx];
                            charge_sum = charge_sum.wrapping_add(physics::weight_to_charge(weight));
                        }
                    }
                }
            }
        }
        expected_i_in[s] = charge_sum;
    }

    // Soma 0 expects charge 1000:
    // Head (9) - seg (5) = 4 < propagation_length (5) -> hit!
    assert_eq!(expected_i_in[0], 1000);
    assert_eq!(out_i_in[0], 1000);

    // Soma 1 expects charge 0:
    // Head (9) - seg (5) = 4 >= propagation_length (3) -> miss!
    assert_eq!(expected_i_in[1], 0);
    assert_eq!(out_i_in[1], 0);

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
#[allow(clippy::identity_op, clippy::erasing_op)]
fn test_cuda_native_full_single_tick_no_gsop_pipeline() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(2).unwrap();

    // Setup base SoA state planes
    let mut voltages = vec![-70_000i32; 64];
    let mut flags = vec![0u8; 64];
    let thresh_offsets = vec![0i32; 64];
    let timers = vec![0u8; 64];
    let mut soma_to_axon = vec![0xFFFFFFFFu32; 64];

    // Variant parameters
    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };

    // Variant 1: Heartbeat-only
    let mut variant_1 = variant_0;
    variant_1.heartbeat_m = physics::constants::MAX_HEARTBEAT_M;

    let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
    variant_table[1] = variant_1;

    // Soma 0: GLIF from virtual input (local axon 0, global 100)
    // Initialize below threshold to prove causality: -60,000 (threshold is -50,000)
    voltages[0] = -60_000;
    soma_to_axon[0] = 0; // mapped to Axon 0

    // Soma 1: GLIF from incoming spike (local axon 1)
    // Initialize below threshold to prove causality: -60,000 (threshold is -50,000)
    voltages[1] = -60_000;
    soma_to_axon[1] = 1; // mapped to Axon 1

    // Soma 2: Heartbeat-only spike
    voltages[2] = -70_000;
    flags[2] = types::SomaFlags::new(false, 0, 1).0; // Type 1 (Heartbeat)

    // Setup synapses:
    let mut dendrite_targets = vec![0u32; 64 * 128];
    let mut dendrite_weights = vec![0i32; 64 * 128];

    // Synapse for Soma 0: connected to virtual axon 100, segment 0 (mapped to local axon 0)
    let target_0 = types::PackedTarget::pack(0, 0).0;
    dendrite_targets[0 * 64 + 0] = target_0;
    dendrite_weights[0 * 64 + 0] = 15_000 << 16; // weight = 15,000, charge = 15,000

    // Synapse for Soma 1: connected to physical axon 1, segment 0
    let target_1 = types::PackedTarget::pack(1, 0).0;
    dendrite_targets[0 * 64 + 1] = target_1;
    dendrite_weights[0 * 64 + 1] = 15_000 << 16; // weight = 15,000, charge = 15,000

    let offsets = layout::compute_state_offsets(64);
    let mut test_state = vec![0u8; state_size];
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_thresh..offsets.off_thresh + 256]
        .copy_from_slice(bytemuck::cast_slice(&thresh_offsets));
    test_state[offsets.off_timers..offsets.off_timers + 64].copy_from_slice(&timers);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));
    test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    // Axons blob initialization (header + empty BurstHeads8)
    let header = layout::AxonsFileHeader::new(2);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let heads = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
    test_axons[16..16 + 2 * 32].copy_from_slice(bytemuck::cast_slice(&heads));

    // ==========================================
    // PHASE 1: Negative Control (No inputs)
    // ==========================================
    {
        let handle = backend.alloc_shard(spec).unwrap();
        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        let mapped_soma_ids = vec![0u32, 1u32, 2u32];
        let max_spikes_per_tick = 3;
        let mut output_spikes = vec![0u32; max_spikes_per_tick as usize];
        let mut output_spike_counts = vec![0u32; 1];

        // Run tick with no virtual or physical incoming spikes
        let result = backend
            .run_single_tick_no_gsop_probe_for_test(
                handle,
                1,    // tick
                2,    // v_seg
                100,  // cmd_virtual_offset
                32,   // num_virtual_axons
                None, // Option<&[u32]> bitmask
                None, // Option<&[u32]> incoming
                &mapped_soma_ids,
                max_spikes_per_tick,
                &mut output_spikes,
                &mut output_spike_counts,
            )
            .unwrap();

        // Expecting ONLY Soma 2 (Heartbeat) to spike. Soma 0 & 1 must remain silent.
        assert_eq!(result.generated_spikes_count, 1);
        assert_eq!(result.output_spikes_written, 1);
        assert_eq!(result.dropped_spikes_count, 0);
        assert_eq!(output_spike_counts[0], 1);
        assert_eq!(output_spikes[0], 2); // Soma 2

        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        // Verify Soma 0 & 1 voltages are decayed and DID NOT reset
        let snap_voltages: &[i32] =
            bytemuck::cast_slice(&snap_state[offsets.off_voltage..offsets.off_voltage + 256]);
        let expected_s0_v_decay = physics::update_glif_voltage(
            voltages[0],
            0,
            variant_0.rest_potential,
            0,
            variant_0.leak_shift as i32,
            0,
            0,
            0,
        );
        assert_eq!(snap_voltages[0], expected_s0_v_decay);
        assert_eq!(snap_voltages[1], expected_s0_v_decay);

        // Verify no head push occurred for Axon 0 and Axon 1
        let mut snap_heads = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
        unsafe {
            std::ptr::copy_nonoverlapping(
                snap_axons[16..16 + 2 * 32].as_ptr(),
                snap_heads.as_mut_ptr() as *mut u8,
                2 * 32,
            );
        }
        assert_eq!(snap_heads[0].h0, types::AXON_SENTINEL);
        assert_eq!(snap_heads[1].h0, types::AXON_SENTINEL);

        backend.free_shard(handle).unwrap();
    }

    // ==========================================
    // PHASE 2: Positive Control (With inputs triggering GLIF spikes)
    // ==========================================
    {
        let handle = backend.alloc_shard(spec).unwrap();
        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        let input_bitmask = vec![0x00000001u32]; // virtual global axon 100 spikes (local 0)
        let incoming_spikes = vec![1u32]; // incoming physical axon 1 spikes
        let mapped_soma_ids = vec![0u32, 1u32, 2u32];
        let max_spikes_per_tick = 2; // only 2 outputs capacity, 3rd will be dropped!
        let mut output_spikes = vec![0u32; max_spikes_per_tick as usize];
        let mut output_spike_counts = vec![0u32; 1];

        let result = backend
            .run_single_tick_no_gsop_probe_for_test(
                handle,
                1,   // tick
                2,   // v_seg
                100, // cmd_virtual_offset
                32,  // num_virtual_axons
                Some(&input_bitmask),
                Some(&incoming_spikes),
                &mapped_soma_ids,
                max_spikes_per_tick,
                &mut output_spikes,
                &mut output_spike_counts,
            )
            .unwrap();

        // Expecting 3 generated spikes (Soma 0, 1, 2)
        assert_eq!(result.generated_spikes_count, 3);
        // Expecting 2 written spikes due to max_spikes_per_tick limit
        assert_eq!(result.output_spikes_written, 2);
        assert_eq!(result.dropped_spikes_count, 1);
        assert_eq!(result.ticks_executed, 1);

        assert_eq!(output_spike_counts[0], 2);
        assert_eq!(output_spikes[0], 0); // Soma 0
        assert_eq!(output_spikes[1], 1); // Soma 1

        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        // Verify weights snapshot is byte-exact (no GSOP changes)
        let snap_weights: &[i32] = bytemuck::cast_slice(
            &snap_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4],
        );
        assert_eq!(snap_weights, &dendrite_weights);

        // Verify Soma 0 & 1 voltages are reset after spike
        let snap_voltages: &[i32] =
            bytemuck::cast_slice(&snap_state[offsets.off_voltage..offsets.off_voltage + 256]);
        let expected_reset =
            (variant_0.rest_potential as u32).wrapping_sub(variant_0.ahp_amplitude as u32) as i32;
        assert_eq!(snap_voltages[0], expected_reset);
        assert_eq!(snap_voltages[1], expected_reset);

        // Verify axon propagation heads have new head pushes
        let mut snap_heads = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
        unsafe {
            std::ptr::copy_nonoverlapping(
                snap_axons[16..16 + 2 * 32].as_ptr(),
                snap_heads.as_mut_ptr() as *mut u8,
                2 * 32,
            );
        }
        let expected_head = physics::initial_axon_head(2);
        assert_eq!(snap_heads[0].h0, expected_head); // Axon 0 received head push from Soma 0
        assert_eq!(snap_heads[1].h0, expected_head); // Axon 1 received head push from Soma 1

        backend.free_shard(handle).unwrap();
    }
}

#[test]
#[cfg(feature = "native")]
#[allow(clippy::identity_op, clippy::erasing_op, clippy::needless_range_loop)]
fn test_cuda_native_gsop_plasticity_probe() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 3,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(3).unwrap();
    let offsets = layout::compute_state_offsets(64);

    // Setup Variant Parameters
    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 200,
        gsop_depression: 150,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [100, 90, 80, 70, 60, 50, 40, 30],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 128, // D1 receptor affinity (LTP boost)
        d2_affinity: 64,  // D2 receptor affinity (LTD suppression)
        heartbeat_m: 0,
    };
    let variant_table = [variant_0; layout::VARIANT_LUT_LEN];

    // Somas flags and targets setup
    let mut flags = vec![0u8; 64];
    let mut dendrite_targets = vec![0u32; 64 * 128];
    let mut dendrite_weights = vec![0i32; 64 * 128];

    // Soma 0: Spiking, Burst=1, Var=0
    flags[0] = types::SomaFlags::new(true, 1, 0).0;
    // Synapse 0: hit
    dendrite_targets[0 * 64 + 0] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0 * 64 + 0] = 1000;
    // Synapse 1: miss
    dendrite_targets[1 * 64 + 0] = types::PackedTarget::pack(1, 10).0;
    dendrite_weights[1 * 64 + 0] = 1000;

    // Soma 1: Non-spiking, Burst=1, Var=0
    flags[1] = types::SomaFlags::new(false, 1, 0).0;
    dendrite_targets[0 * 64 + 1] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0 * 64 + 1] = 1000;

    // Soma 2: Spiking, Burst=3, Var=0 (Burst multiplier check)
    flags[2] = types::SomaFlags::new(true, 3, 0).0;
    dendrite_targets[0 * 64 + 2] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0 * 64 + 2] = 1000;

    // Soma 3: Spiking, Burst=1, Var=0 (Negative weights / Dale's law)
    flags[3] = types::SomaFlags::new(true, 1, 0).0;
    // Synapse 0: hit, negative weight
    dendrite_targets[0 * 64 + 3] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0 * 64 + 3] = -1000;
    // Synapse 1: miss, negative weight close to limit
    dendrite_targets[1 * 64 + 3] = types::PackedTarget::pack(1, 10).0;
    dendrite_weights[1 * 64 + 3] = -2;

    // Soma 4: Spiking, Burst=1, Var=0 (Out of range / inactive / corrupt check)
    flags[4] = types::SomaFlags::new(true, 1, 0).0;
    // Synapse 0: out of range axon
    dendrite_targets[0 * 64 + 4] = types::PackedTarget::pack(99, 0).0;
    dendrite_weights[0 * 64 + 4] = 1000;
    // Synapse 1: NONE
    dendrite_targets[1 * 64 + 4] = types::PackedTarget::NONE.0;
    dendrite_weights[1 * 64 + 4] = 1000;
    // Synapse 2: TOMBSTONE
    dendrite_targets[2 * 64 + 4] = types::PackedTarget::TOMBSTONE.0;
    dendrite_weights[2 * 64 + 4] = 1000;
    // Synapse 3: Corrupt/reserved target encoding (axon_q > MAX_AXON_ID + 1)
    dendrite_targets[3 * 64 + 4] = 0x00FFFFFF;
    dendrite_weights[3 * 64 + 4] = 1000;

    // Soma 5: Spiking, Burst=1, Var=0 (i32::MIN check)
    flags[5] = types::SomaFlags::new(true, 1, 0).0;
    dendrite_targets[0 * 64 + 5] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0 * 64 + 5] = i32::MIN;

    // Soma 6: Spiking, Burst=1, Var=0 (floor check positive)
    flags[6] = types::SomaFlags::new(true, 1, 0).0;
    dendrite_targets[0 * 64 + 6] = types::PackedTarget::pack(1, 10).0; // miss
    dendrite_weights[0 * 64 + 6] = 2;

    // Setup Axon heads:
    let header = layout::AxonsFileHeader::new(3);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let mut heads = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 3];
    heads[0].h0 = 0;
    heads[1].h0 = 0;
    heads[2].h0 = 0;
    test_axons[16..16 + 3 * 32].copy_from_slice(bytemuck::cast_slice(&heads));

    // Dopamine cases to test
    let dopamine_levels = vec![0, 50, -50];

    for &dopamine in &dopamine_levels {
        let handle = backend.alloc_shard(spec).unwrap();

        let mut test_state = vec![0u8; state_size];
        test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
        test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
            .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
        test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
            .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        // Run GSOP probe
        backend
            .apply_gsop_plasticity_probe_for_test(handle, dopamine)
            .unwrap();

        // Download snapshot
        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        let snap_weights: &[i32] = bytemuck::cast_slice(
            &snap_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4],
        );

        // Verify all 64 * 128 weights against physics::apply_gsop_plasticity
        for s in 0..64 {
            let soma_flags = types::SomaFlags(flags[s]);
            let spiking = soma_flags.spiking();
            let burst_count = soma_flags.burst_count();
            let variant_idx = soma_flags.type_id() as usize;
            let var = variant_table[variant_idx];

            for d in 0..128 {
                let idx = d * 64 + s;
                let w_old = dendrite_weights[idx];
                let w_new = snap_weights[idx];

                if !spiking {
                    assert_eq!(
                        w_new, w_old,
                        "Non-spiking soma weight changed at s={}, d={}",
                        s, d
                    );
                    continue;
                }

                let raw_target = dendrite_targets[idx];
                let target = types::PackedTarget(raw_target);
                if target.is_inactive() {
                    assert_eq!(
                        w_new, w_old,
                        "Inactive target weight changed at s={}, d={}",
                        s, d
                    );
                    continue;
                }

                if let Some((axon_id, segment_index)) = target.unpack() {
                    if axon_id >= spec.total_axons {
                        assert_eq!(
                            w_new, w_old,
                            "Out-of-range target weight changed at s={}, d={}",
                            s, d
                        );
                        continue;
                    }

                    // Compute hit: dist < propagation_length
                    let dist = heads[axon_id as usize].h0.wrapping_sub(segment_index);
                    let is_active = dist < var.signal_propagation_length as u32;

                    let mut curve_i32 = [0i32; 8];
                    for i in 0..8 {
                        curve_i32[i] = var.inertia_curve[i] as i32;
                    }

                    let expected_w = physics::apply_gsop_plasticity(
                        w_old,
                        is_active,
                        var.gsop_potentiation as i32,
                        var.gsop_depression as i32,
                        dopamine,
                        var.d1_affinity as i32,
                        var.d2_affinity as i32,
                        burst_count as u32,
                        &curve_i32,
                    );

                    assert_eq!(
                        w_new, expected_w,
                        "Weight mismatch at s={}, d={}, dopamine={}. Expected {}, got {}",
                        s, d, dopamine, expected_w, w_new
                    );
                } else {
                    assert_eq!(
                        w_new, w_old,
                        "Corrupt target weight changed at s={}, d={}",
                        s, d
                    );
                }
            }
        }

        backend.free_shard(handle).unwrap();
    }
}

#[test]
#[cfg(feature = "native")]
#[allow(clippy::identity_op, clippy::erasing_op, clippy::needless_range_loop)]
fn test_cuda_native_full_single_tick_with_gsop_pipeline() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(2).unwrap();
    let offsets = layout::compute_state_offsets(64);

    // 1. Setup Base State
    let mut voltages = vec![-70_000i32; 64];
    let mut flags = vec![0u8; 64];
    let thresh_offsets = vec![0i32; 64];
    let timers = vec![0u8; 64];
    let mut soma_to_axon = vec![0xFFFFFFFFu32; 64];

    // Variant parameters
    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 200,
        gsop_depression: 150,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [100, 90, 80, 70, 60, 50, 40, 30],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 128,
        d2_affinity: 64,
        heartbeat_m: 0,
    };

    // Variant 1: Heartbeat-only
    let mut variant_1 = variant_0;
    variant_1.heartbeat_m = physics::constants::MAX_HEARTBEAT_M;

    let mut variant_table = [variant_0; layout::VARIANT_LUT_LEN];
    variant_table[1] = variant_1;

    // Soma 0: GLIF from virtual input (local axon 0, global 100)
    voltages[0] = -60_000;
    soma_to_axon[0] = 0;

    // Soma 1: GLIF from incoming spike (local axon 1)
    voltages[1] = -60_000;
    soma_to_axon[1] = 1;

    // Soma 2: Heartbeat-only
    voltages[2] = -70_000;
    flags[2] = types::SomaFlags::new(false, 0, 1).0; // Type 1

    // Soma 3: Non-spiking, but active target check
    voltages[3] = -70_000;

    // Setup synapses:
    let mut dendrite_targets = vec![0u32; 64 * 128];
    let mut dendrite_weights = vec![0i32; 64 * 128];

    // --- Inputs/Current slots (Soma 0 & Soma 1 spiking triggers) ---
    // Soma 0 receives current from virtual input (local axon 0 segment 0)
    dendrite_targets[0 * 64 + 0] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0 * 64 + 0] = 15_000 << 16; // 15,000 mass -> triggers GLIF spike
                                                 // Soma 1 receives current from incoming physical spike (local axon 1 segment 0)
    dendrite_targets[0 * 64 + 1] = types::PackedTarget::pack(1, 0).0;
    dendrite_weights[0 * 64 + 1] = 15_000 << 16; // 15,000 mass -> triggers GLIF spike

    // --- Non-spiking soma check (Soma 3) ---
    dendrite_targets[0 * 64 + 3] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0 * 64 + 3] = 1000; // not enough to spike, but active target

    // --- GSOP slots for Soma 0 (Spiking) ---
    // Slot 2: positive weight hit -> LTP potentiation
    dendrite_targets[2 * 64 + 0] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[2 * 64 + 0] = 1000;
    // Slot 3: positive weight miss -> LTD depression
    dendrite_targets[3 * 64 + 0] = types::PackedTarget::pack(1, 10).0;
    dendrite_weights[3 * 64 + 0] = 1000;
    // Slot 4: negative weight hit -> Dale's law preserved (potentiation)
    dendrite_targets[4 * 64 + 0] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[4 * 64 + 0] = -1000;
    // Slot 5: negative weight miss floor check -> clamp to -1
    dendrite_targets[5 * 64 + 0] = types::PackedTarget::pack(1, 10).0;
    dendrite_weights[5 * 64 + 0] = -2;
    // Slot 6: corrupt target check (axon_q > MAX_AXON_ID + 1)
    dendrite_targets[6 * 64 + 0] = 0x00FFFFFF;
    dendrite_weights[6 * 64 + 0] = 1000;
    // Slot 7: inactive target NONE check
    dendrite_targets[7 * 64 + 0] = types::PackedTarget::NONE.0;
    dendrite_weights[7 * 64 + 0] = 1000;
    // Slot 8: inactive target TOMBSTONE check
    dendrite_targets[8 * 64 + 0] = types::PackedTarget::TOMBSTONE.0;
    dendrite_weights[8 * 64 + 0] = 1000;
    // Slot 9: out of range target check (axon >= 2)
    dendrite_targets[9 * 64 + 0] = types::PackedTarget::pack(99, 0).0;
    dendrite_weights[9 * 64 + 0] = 1000;

    let mut test_state = vec![0u8; state_size];
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_thresh..offsets.off_thresh + 256]
        .copy_from_slice(bytemuck::cast_slice(&thresh_offsets));
    test_state[offsets.off_timers..offsets.off_timers + 64].copy_from_slice(&timers);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));
    test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    // Axons setup:
    let header = layout::AxonsFileHeader::new(2);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let heads_init = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
    test_axons[16..16 + 2 * 32].copy_from_slice(bytemuck::cast_slice(&heads_init));

    // Inputs setup:
    let current_tick = 1;
    let v_seg = 2;
    let cmd_virtual_offset = 100;
    let num_virtual_axons = 32;
    let input_bitmask = vec![0x00000001u32]; // virtual global axon 100 spikes (local 0)
    let incoming_spikes = vec![1u32]; // incoming physical axon 1 spikes
    let mapped_soma_ids = vec![0u32, 1u32, 2u32];
    let max_spikes_per_tick = 2; // only 2 outputs capacity, 3rd (soma 2) is dropped!
    let dopamine = 50;

    // ==========================================
    // PHASE 1: Negative Comparison (No GSOP)
    // ==========================================
    {
        let handle = backend.alloc_shard(spec).unwrap();
        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        let mut output_spikes = vec![0u32; max_spikes_per_tick as usize];
        let mut output_spike_counts = vec![0u32; 1];

        // Run no-GSOP single tick pipeline
        backend
            .run_single_tick_no_gsop_probe_for_test(
                handle,
                current_tick,
                v_seg,
                cmd_virtual_offset,
                num_virtual_axons,
                Some(&input_bitmask),
                Some(&incoming_spikes),
                &mapped_soma_ids,
                max_spikes_per_tick,
                &mut output_spikes,
                &mut output_spike_counts,
            )
            .unwrap();

        // Download snapshot
        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        // Verify that weights did not change at all
        let snap_weights: &[i32] = bytemuck::cast_slice(
            &snap_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4],
        );
        assert_eq!(
            snap_weights, &dendrite_weights,
            "Weights changed during no-GSOP pipeline!"
        );

        backend.free_shard(handle).unwrap();
    }

    // ==========================================
    // PHASE 2: Positive Control (With GSOP)
    // ==========================================
    {
        let handle = backend.alloc_shard(spec).unwrap();
        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        let mut output_spikes = vec![0u32; max_spikes_per_tick as usize];
        let mut output_spike_counts = vec![0u32; 1];

        // Run with-GSOP single tick pipeline
        let result = backend
            .run_single_tick_with_gsop_probe_for_test(
                handle,
                current_tick,
                v_seg,
                cmd_virtual_offset,
                num_virtual_axons,
                Some(&input_bitmask),
                Some(&incoming_spikes),
                &mapped_soma_ids,
                max_spikes_per_tick,
                &mut output_spikes,
                &mut output_spike_counts,
                dopamine,
            )
            .unwrap();

        // 1. Verify BatchResult
        assert_eq!(result.generated_spikes_count, 3);
        assert_eq!(result.output_spikes_written, 2);
        assert_eq!(result.dropped_spikes_count, 1);
        assert_eq!(result.ticks_executed, 1);

        assert_eq!(output_spike_counts[0], 2);
        assert_eq!(output_spikes[0], 0); // Soma 0
        assert_eq!(output_spikes[1], 1); // Soma 1

        // Download snapshot
        let mut snap_state = vec![0u8; state_size];
        let mut snap_axons = vec![0u8; axons_size];
        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        // 2. Verify voltage/timer/flags for GLIF and heartbeat
        let snap_voltages: &[i32] =
            bytemuck::cast_slice(&snap_state[offsets.off_voltage..offsets.off_voltage + 256]);
        let snap_flags: &[u8] = &snap_state[offsets.off_flags..offsets.off_flags + 64];

        let expected_reset =
            (variant_0.rest_potential as u32).wrapping_sub(variant_0.ahp_amplitude as u32) as i32;
        assert_eq!(snap_voltages[0], expected_reset);
        assert_eq!(snap_voltages[1], expected_reset);
        assert_eq!(snap_voltages[2], variant_1.rest_potential); // Heartbeat only decays/stays at rest

        let f0 = types::SomaFlags(snap_flags[0]);
        let f1 = types::SomaFlags(snap_flags[1]);
        let f2 = types::SomaFlags(snap_flags[2]);
        let f3 = types::SomaFlags(snap_flags[3]);

        assert!(f0.spiking());
        assert_eq!(f0.burst_count(), 1);
        assert!(f1.spiking());
        assert_eq!(f1.burst_count(), 1);
        assert!(f2.spiking());
        assert_eq!(f2.burst_count(), 1);
        assert!(!f3.spiking());
        assert_eq!(f3.burst_count(), 0);

        // 3. Verify axon heads after final spike
        let mut snap_heads = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
        unsafe {
            std::ptr::copy_nonoverlapping(
                snap_axons[16..16 + 2 * 32].as_ptr(),
                snap_heads.as_mut_ptr() as *mut u8,
                2 * 32,
            );
        }
        let expected_head = physics::initial_axon_head(v_seg);
        assert_eq!(snap_heads[0].h0, expected_head); // Axon 0 received head push from Soma 0
        assert_eq!(snap_heads[1].h0, expected_head); // Axon 1 received head push from Soma 1

        // 4. Verify weights after GSOP
        let snap_weights: &[i32] = bytemuck::cast_slice(
            &snap_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4],
        );

        for s in 0..64 {
            let sf = types::SomaFlags(snap_flags[s]);
            let spiking = sf.spiking();
            let burst_count = sf.burst_count();
            let variant_idx = sf.type_id() as usize;
            let var = variant_table[variant_idx];

            let mut curve_i32 = [0i32; 8];
            for i in 0..8 {
                curve_i32[i] = var.inertia_curve[i] as i32;
            }

            for d in 0..128 {
                let idx = d * 64 + s;
                let w_old = dendrite_weights[idx];
                let w_new = snap_weights[idx];

                if !spiking {
                    assert_eq!(
                        w_new, w_old,
                        "Non-spiking soma weight changed at s={}, d={}",
                        s, d
                    );
                    continue;
                }

                let raw_target = dendrite_targets[idx];
                let target = types::PackedTarget(raw_target);
                if target.is_inactive() {
                    assert_eq!(
                        w_new, w_old,
                        "Inactive target weight changed at s={}, d={}",
                        s, d
                    );
                    continue;
                }

                if let Some((axon_id, segment_index)) = target.unpack() {
                    if axon_id >= spec.total_axons {
                        assert_eq!(
                            w_new, w_old,
                            "Out-of-range target weight changed at s={}, d={}",
                            s, d
                        );
                        continue;
                    }

                    // Compute hit: axon heads were updated BEFORE GSOP plasticity!
                    // So we look at all 8 updated heads in snap_heads!
                    let mut is_active = false;
                    let heads_array = [
                        snap_heads[axon_id as usize].h0,
                        snap_heads[axon_id as usize].h1,
                        snap_heads[axon_id as usize].h2,
                        snap_heads[axon_id as usize].h3,
                        snap_heads[axon_id as usize].h4,
                        snap_heads[axon_id as usize].h5,
                        snap_heads[axon_id as usize].h6,
                        snap_heads[axon_id as usize].h7,
                    ];
                    for h_val in heads_array {
                        let dist = h_val.wrapping_sub(segment_index);
                        if dist < var.signal_propagation_length as u32 {
                            is_active = true;
                            break;
                        }
                    }

                    let expected_w = physics::apply_gsop_plasticity(
                        w_old,
                        is_active,
                        var.gsop_potentiation as i32,
                        var.gsop_depression as i32,
                        dopamine,
                        var.d1_affinity as i32,
                        var.d2_affinity as i32,
                        burst_count as u32,
                        &curve_i32,
                    );

                    assert_eq!(
                        w_new, expected_w,
                        "Weight mismatch at s={}, d={}. Expected {}, got {}",
                        s, d, expected_w, w_new
                    );
                } else {
                    assert_eq!(
                        w_new, w_old,
                        "Corrupt target weight changed at s={}, d={}",
                        s, d
                    );
                }
            }
        }

        backend.free_shard(handle).unwrap();
    }
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_run_day_batch_single_tick_matches_full_probe() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(2).unwrap();
    let offsets = layout::compute_state_offsets(64);

    let mut voltages = vec![-70_000i32; 64];
    voltages[0] = -60_000; // will spike
    let flags = vec![0u8; 64];
    let mut soma_to_axon = vec![0xFFFFFFFFu32; 64];
    soma_to_axon[0] = 0;

    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 200,
        gsop_depression: 150,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [100, 90, 80, 70, 60, 50, 40, 30],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 128,
        d2_affinity: 64,
        heartbeat_m: 0,
    };
    let variant_table = [variant_0; layout::VARIANT_LUT_LEN];

    let mut dendrite_targets = vec![0u32; 64 * 128];
    let mut dendrite_weights = vec![0i32; 64 * 128];
    dendrite_targets[0] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0] = 15_000 << 16; // current input to trigger spike

    let mut test_state = vec![0u8; state_size];
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));
    test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    let header = layout::AxonsFileHeader::new(2);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let heads_init = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
    test_axons[16..16 + 2 * 32].copy_from_slice(bytemuck::cast_slice(&heads_init));

    let input_bitmask = vec![0x00000001u32];
    let mapped_soma_ids = vec![0u32];
    let max_spikes_per_tick = 2;
    let dopamine = 50;

    let mut output_spikes_1 = vec![0u32; max_spikes_per_tick as usize];
    let mut output_spike_counts_1 = vec![0u32; 1];
    let mut snap_state_1 = vec![0u8; state_size];
    let mut snap_axons_1 = vec![0u8; axons_size];
    let res_1;

    // Run direct probe
    {
        let handle = backend.alloc_shard(spec).unwrap();
        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        res_1 = backend
            .run_single_tick_with_gsop_probe_for_test(
                handle,
                1,
                2,
                100,
                32,
                Some(&input_bitmask),
                None,
                &mapped_soma_ids,
                max_spikes_per_tick,
                &mut output_spikes_1,
                &mut output_spike_counts_1,
                dopamine,
            )
            .unwrap();

        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state_1,
                    axons_blob: &mut snap_axons_1,
                },
            )
            .unwrap();

        backend.free_shard(handle).unwrap();
    }

    let mut output_spikes_2 = vec![0u32; max_spikes_per_tick as usize];
    let mut output_spike_counts_2 = vec![0u32; 1];
    let mut snap_state_2 = vec![0u8; state_size];
    let mut snap_axons_2 = vec![0u8; axons_size];
    let res_2;

    // Run via run_day_batch
    {
        let handle = backend.alloc_shard(spec).unwrap();
        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        backend.upload_shard(handle, upload).unwrap();

        use compute_api::ComputeBackend;
        let cmd = compute_api::DayBatchCmd {
            tick_base: 1,
            sync_batch_ticks: 1,
            v_seg: 2,
            virtual_offset: 100,
            num_virtual_axons: 32,
            input_words_per_tick: 1,
            max_spikes_per_tick,
            num_outputs: mapped_soma_ids.len() as u32,
            dopamine: dopamine as i16,
            input_bitmask: Some(&input_bitmask),
            incoming_spikes: None,
            incoming_spike_counts: &[0],
            mapped_soma_ids: &mapped_soma_ids,
            output_spikes: &mut output_spikes_2,
            output_spike_counts: &mut output_spike_counts_2,
        };

        res_2 = backend.run_day_batch(handle, cmd).unwrap();

        backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state_2,
                    axons_blob: &mut snap_axons_2,
                },
            )
            .unwrap();

        backend.free_shard(handle).unwrap();
    }

    // Verify equality
    assert_eq!(res_1.ticks_executed, res_2.ticks_executed);
    assert_eq!(res_1.generated_spikes_count, res_2.generated_spikes_count);
    assert_eq!(res_1.output_spikes_written, res_2.output_spikes_written);
    assert_eq!(res_1.dropped_spikes_count, res_2.dropped_spikes_count);

    assert_eq!(output_spike_counts_1, output_spike_counts_2);
    assert_eq!(output_spikes_1, output_spikes_2);

    assert_eq!(snap_state_1, snap_state_2, "State blob mismatch!");
    assert_eq!(snap_axons_1, snap_axons_2, "Axons blob mismatch!");
}

#[test]
#[cfg(feature = "native")]
#[allow(clippy::needless_range_loop)]
fn test_cuda_native_run_day_batch_multi_tick_cpu_differential() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(2).unwrap();
    let offsets = layout::compute_state_offsets(64);

    let mut voltages = vec![-70_000i32; 64];
    voltages[0] = -60_000; // will spike
    let flags = vec![0u8; 64];
    let mut soma_to_axon = vec![0xFFFFFFFFu32; 64];
    soma_to_axon[0] = 0;

    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 200,
        gsop_depression: 150,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [100, 90, 80, 70, 60, 50, 40, 30],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 128,
        d2_affinity: 64,
        heartbeat_m: 0,
    };
    let variant_table = [variant_0; layout::VARIANT_LUT_LEN];

    let mut dendrite_targets = vec![0u32; 64 * 128];
    let mut dendrite_weights = vec![0i32; 64 * 128];
    dendrite_targets[0] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0] = 15_000 << 16;
    dendrite_targets[1] = types::PackedTarget::pack(1, 0).0;
    dendrite_weights[1] = 15_000 << 16;

    let mut test_state = vec![0u8; state_size];
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));
    test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    let header = layout::AxonsFileHeader::new(2);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let heads_init = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
    test_axons[16..16 + 2 * 32].copy_from_slice(bytemuck::cast_slice(&heads_init));

    let input_bitmask = vec![
        0x00000001u32, // tick 1: virtual axon 100 spikes (local 0)
        0x00000000u32, // tick 2
        0x00000001u32, // tick 3: virtual axon 100 spikes
    ];
    let incoming_spikes = vec![
        1u32, 0u32, // tick 1
        1u32, 0u32, // tick 2
        1u32, 0u32, // tick 3
    ];
    let incoming_spike_counts = vec![1u32, 1u32, 1u32];
    let mapped_soma_ids = vec![0u32, 1u32];
    let max_spikes_per_tick = 2;
    let sync_batch_ticks = 3;
    let dopamine = 50;

    let mut output_spikes_cpu = vec![0u32; (max_spikes_per_tick * sync_batch_ticks) as usize];
    let mut output_spike_counts_cpu = vec![0u32; sync_batch_ticks as usize];
    let mut snap_state_cpu = vec![0u8; state_size];
    let mut snap_axons_cpu = vec![0u8; axons_size];
    let res_cpu;

    // 1. Run CPU Backend
    {
        let mut cpu_backend =
            compute_cpu::CpuBackend::new(compute_cpu::CpuBackendConfig::default()).unwrap();
        use compute_api::ComputeBackend;
        let handle = cpu_backend.alloc_shard(spec).unwrap();
        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        cpu_backend.upload_shard(handle, upload).unwrap();

        let cmd = compute_api::DayBatchCmd {
            tick_base: 1,
            sync_batch_ticks,
            v_seg: 2,
            virtual_offset: 100,
            num_virtual_axons: 32,
            input_words_per_tick: 1,
            max_spikes_per_tick,
            num_outputs: mapped_soma_ids.len() as u32,
            dopamine: dopamine as i16,
            input_bitmask: Some(&input_bitmask),
            incoming_spikes: Some(&incoming_spikes),
            incoming_spike_counts: &incoming_spike_counts,
            mapped_soma_ids: &mapped_soma_ids,
            output_spikes: &mut output_spikes_cpu,
            output_spike_counts: &mut output_spike_counts_cpu,
        };

        res_cpu = cpu_backend.run_day_batch(handle, cmd).unwrap();

        cpu_backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state_cpu,
                    axons_blob: &mut snap_axons_cpu,
                },
            )
            .unwrap();

        cpu_backend.free_shard(handle).unwrap();
    }

    let mut output_spikes_cuda = vec![0u32; (max_spikes_per_tick * sync_batch_ticks) as usize];
    let mut output_spike_counts_cuda = vec![0u32; sync_batch_ticks as usize];
    let mut snap_state_cuda = vec![0u8; state_size];
    let mut snap_axons_cuda = vec![0u8; axons_size];
    let res_cuda;

    // 2. Run CUDA Backend
    {
        let mut cuda_backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();
        use compute_api::ComputeBackend;
        let handle = cuda_backend.alloc_shard(spec).unwrap();
        let upload = compute_api::ShardUpload {
            state_blob: &test_state,
            axons_blob: &test_axons,
            variant_table: &variant_table,
        };
        cuda_backend.upload_shard(handle, upload).unwrap();

        let cmd = compute_api::DayBatchCmd {
            tick_base: 1,
            sync_batch_ticks,
            v_seg: 2,
            virtual_offset: 100,
            num_virtual_axons: 32,
            input_words_per_tick: 1,
            max_spikes_per_tick,
            num_outputs: mapped_soma_ids.len() as u32,
            dopamine: dopamine as i16,
            input_bitmask: Some(&input_bitmask),
            incoming_spikes: Some(&incoming_spikes),
            incoming_spike_counts: &incoming_spike_counts,
            mapped_soma_ids: &mapped_soma_ids,
            output_spikes: &mut output_spikes_cuda,
            output_spike_counts: &mut output_spike_counts_cuda,
        };

        res_cuda = cuda_backend.run_day_batch(handle, cmd).unwrap();

        cuda_backend
            .debug_snapshot(
                handle,
                compute_api::ShardSnapshotMut {
                    state_blob: &mut snap_state_cuda,
                    axons_blob: &mut snap_axons_cuda,
                },
            )
            .unwrap();

        cuda_backend.free_shard(handle).unwrap();
    }

    // Verify differential equality
    assert_eq!(res_cpu.ticks_executed, res_cuda.ticks_executed);
    assert_eq!(
        res_cpu.generated_spikes_count,
        res_cuda.generated_spikes_count
    );
    assert_eq!(
        res_cpu.output_spikes_written,
        res_cuda.output_spikes_written
    );
    assert_eq!(res_cpu.dropped_spikes_count, res_cuda.dropped_spikes_count);

    assert_eq!(output_spike_counts_cpu, output_spike_counts_cuda);

    // Verify used regions of output spikes
    for tick in 0..sync_batch_ticks as usize {
        let count = output_spike_counts_cpu[tick] as usize;
        let start = tick * max_spikes_per_tick as usize;
        assert_eq!(
            &output_spikes_cpu[start..start + count],
            &output_spikes_cuda[start..start + count],
            "Output spikes mismatch for tick {}",
            tick
        );
    }

    assert_eq!(
        snap_state_cpu, snap_state_cuda,
        "Differential state mismatch!"
    );
    assert_eq!(
        snap_axons_cpu, snap_axons_cuda,
        "Differential axons mismatch!"
    );
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_run_day_batch_validation_errors() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(2).unwrap();
    let offsets = layout::compute_state_offsets(64);

    let voltages = vec![-70_000i32; 64];
    let flags = vec![0u8; 64];
    let soma_to_axon = vec![0xFFFFFFFFu32; 64];
    let dendrite_targets = vec![0u32; 64 * 128];
    let dendrite_weights = vec![0i32; 64 * 128];

    let mut test_state = vec![0u8; state_size];
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));
    test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    let header = layout::AxonsFileHeader::new(2);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let heads_init = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
    test_axons[16..16 + 2 * 32].copy_from_slice(bytemuck::cast_slice(&heads_init));

    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 200,
        gsop_depression: 150,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [100, 90, 80, 70, 60, 50, 40, 30],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 128,
        d2_affinity: 64,
        heartbeat_m: 0,
    };
    let variant_table = [variant_0; layout::VARIANT_LUT_LEN];

    let handle = backend.alloc_shard(spec).unwrap();
    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    let input_bitmask = vec![0x0u32; 10];
    let incoming_spikes = vec![0u32; 10];
    let incoming_spike_counts = vec![0u32; 1];
    let mapped_soma_ids = vec![0u32];

    let mut output_spikes = vec![0u32; 2];
    let mut output_spike_counts = vec![0u32; 1];

    use compute_api::ComputeBackend;

    // Case 1: invalid v_seg = 0
    let cmd_err_1 = compute_api::DayBatchCmd {
        tick_base: 1,
        sync_batch_ticks: 1,
        v_seg: 0,
        virtual_offset: 100,
        num_virtual_axons: 32,
        input_words_per_tick: 1,
        max_spikes_per_tick: 2,
        num_outputs: mapped_soma_ids.len() as u32,
        dopamine: 0,
        input_bitmask: Some(&input_bitmask),
        incoming_spikes: Some(&incoming_spikes),
        incoming_spike_counts: &incoming_spike_counts,
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };
    assert!(backend.run_day_batch(handle, cmd_err_1).is_err());

    // Case 2: insufficient output buffer length
    let mut small_output_spikes = vec![0u32; 1]; // needs 2
    let cmd_err_2 = compute_api::DayBatchCmd {
        tick_base: 1,
        sync_batch_ticks: 1,
        v_seg: 2,
        virtual_offset: 100,
        num_virtual_axons: 32,
        input_words_per_tick: 1,
        max_spikes_per_tick: 2,
        num_outputs: mapped_soma_ids.len() as u32,
        dopamine: 0,
        input_bitmask: Some(&input_bitmask),
        incoming_spikes: Some(&incoming_spikes),
        incoming_spike_counts: &incoming_spike_counts,
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut small_output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };
    assert!(backend.run_day_batch(handle, cmd_err_2).is_err());

    // Case 3: missing incoming spikes but non-zero count
    let counts_err = vec![1u32];
    let cmd_err_3 = compute_api::DayBatchCmd {
        tick_base: 1,
        sync_batch_ticks: 1,
        v_seg: 2,
        virtual_offset: 100,
        num_virtual_axons: 32,
        input_words_per_tick: 1,
        max_spikes_per_tick: 2,
        num_outputs: mapped_soma_ids.len() as u32,
        dopamine: 0,
        input_bitmask: Some(&input_bitmask),
        incoming_spikes: None, // Missing spikes!
        incoming_spike_counts: &counts_err,
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };
    assert!(backend.run_day_batch(handle, cmd_err_3).is_err());

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_run_day_batch_output_counts_reset() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(2).unwrap();
    let offsets = layout::compute_state_offsets(64);

    let voltages = vec![-70_000i32; 64];
    let flags = vec![0u8; 64];
    let soma_to_axon = vec![0xFFFFFFFFu32; 64];
    let dendrite_targets = vec![0u32; 64 * 128];
    let dendrite_weights = vec![0i32; 64 * 128];

    let mut test_state = vec![0u8; state_size];
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));
    test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    let header = layout::AxonsFileHeader::new(2);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let heads_init = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
    test_axons[16..16 + 2 * 32].copy_from_slice(bytemuck::cast_slice(&heads_init));

    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 200,
        gsop_depression: 150,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [100, 90, 80, 70, 60, 50, 40, 30],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 128,
        d2_affinity: 64,
        heartbeat_m: 0,
    };
    let variant_table = [variant_0; layout::VARIANT_LUT_LEN];

    let handle = backend.alloc_shard(spec).unwrap();
    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    let mapped_soma_ids = vec![0u32];

    let mut output_spikes = vec![0u32; 4];
    let mut output_spike_counts = vec![999u32; 2];

    use compute_api::ComputeBackend;

    let cmd = compute_api::DayBatchCmd {
        tick_base: 1,
        sync_batch_ticks: 2,
        v_seg: 2,
        virtual_offset: 100,
        num_virtual_axons: 32,
        input_words_per_tick: 1,
        max_spikes_per_tick: 2,
        num_outputs: mapped_soma_ids.len() as u32,
        dopamine: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[0, 0],
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    let result = backend.run_day_batch(handle, cmd).unwrap();

    assert_eq!(result.ticks_executed, 2);
    assert_eq!(output_spike_counts[0], 0);
    assert_eq!(output_spike_counts[1], 0);

    backend.free_shard(handle).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn test_cuda_native_run_day_batch_short_input_stride_fails() {
    if !is_gpu_available() {
        return;
    }
    let _lock = GPU_TEST_LOCK.lock().unwrap();
    let mut backend = CudaBackend::new(CudaBackendConfig::default()).unwrap();

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::validation::expected_axons_blob_size(2).unwrap();
    let offsets = layout::compute_state_offsets(64);

    let voltages = vec![-70_000i32; 64];
    let flags = vec![0u8; 64];
    let soma_to_axon = vec![0xFFFFFFFFu32; 64];
    let dendrite_targets = vec![0u32; 64 * 128];
    let dendrite_weights = vec![0i32; 64 * 128];

    let mut test_state = vec![0u8; state_size];
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));
    test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    let header = layout::AxonsFileHeader::new(2);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let heads_init = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
    test_axons[16..16 + 2 * 32].copy_from_slice(bytemuck::cast_slice(&heads_init));

    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 200,
        gsop_depression: 150,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [100, 90, 80, 70, 60, 50, 40, 30],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 128,
        d2_affinity: 64,
        heartbeat_m: 0,
    };
    let variant_table = [variant_0; layout::VARIANT_LUT_LEN];

    let handle = backend.alloc_shard(spec).unwrap();
    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons,
        variant_table: &variant_table,
    };
    backend.upload_shard(handle, upload).unwrap();

    let input_bitmask = vec![0u32; 1];
    let mapped_soma_ids = vec![0u32];

    let mut output_spikes = vec![0u32; 2];
    let mut output_spike_counts = vec![0u32; 1];

    use compute_api::ComputeBackend;

    // num_virtual_axons: 33 -> needs 2 words per tick.
    // Providing input_words_per_tick: 1 must fail with InvalidBatch.
    let cmd = compute_api::DayBatchCmd {
        tick_base: 1,
        sync_batch_ticks: 1,
        v_seg: 2,
        virtual_offset: 100,
        num_virtual_axons: 33,
        input_words_per_tick: 1,
        max_spikes_per_tick: 2,
        num_outputs: mapped_soma_ids.len() as u32,
        dopamine: 0,
        input_bitmask: Some(&input_bitmask),
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    let result = backend.run_day_batch(handle, cmd);
    assert!(result.is_err());

    backend.free_shard(handle).unwrap();
}
