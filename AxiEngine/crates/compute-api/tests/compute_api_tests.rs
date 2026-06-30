use compute_api::*;
use core::num::NonZeroU64;

const ZERO_VARIANT: layout::VariantParameters = layout::VariantParameters {
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

fn dummy_variants() -> [layout::VariantParameters; layout::VARIANT_LUT_LEN] {
    [ZERO_VARIANT; layout::VARIANT_LUT_LEN]
}

struct MockBackend {
    handle: Option<VramHandle>,
    spec: Option<ShardAllocSpec>,
    uploaded: bool,
    freed: bool,
}

impl MockBackend {
    fn new() -> Self {
        Self {
            handle: None,
            spec: None,
            uploaded: false,
            freed: false,
        }
    }
}

impl ComputeBackend for MockBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Mock
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            lane_count: 32,
            supports_async: false,
            supports_ephys: false,
            max_batch_ticks: 1000,
            alignment_bytes: 64,
            pinned_host_required: false,
        }
    }

    fn alloc_shard(&mut self, spec: ShardAllocSpec) -> Result<VramHandle, ComputeApiError> {
        validate_alloc_spec(&spec)?;
        let handle = VramHandle::from_raw_parts(BackendKind::Mock, NonZeroU64::new(42).unwrap(), 1);
        self.handle = Some(handle);
        self.spec = Some(spec);
        self.uploaded = false;
        self.freed = false;
        Ok(handle)
    }

    fn upload_shard(
        &mut self,
        handle: VramHandle,
        upload: ShardUpload<'_>,
    ) -> Result<(), ComputeApiError> {
        if self.freed {
            return Err(ComputeApiError::AlreadyFreed);
        }
        let active = self.handle.ok_or(ComputeApiError::InvalidHandle)?;
        if handle != active {
            if handle.kind() != BackendKind::Mock {
                return Err(ComputeApiError::ForeignHandle);
            }
            return Err(ComputeApiError::InvalidHandle);
        }
        let spec = self.spec.as_ref().ok_or(ComputeApiError::InvalidHandle)?;
        validate_upload(spec, &upload)?;
        self.uploaded = true;
        Ok(())
    }

    fn run_day_batch(
        &mut self,
        handle: VramHandle,
        cmd: DayBatchCmd<'_>,
    ) -> Result<BatchResult, ComputeApiError> {
        if self.freed {
            return Err(ComputeApiError::AlreadyFreed);
        }
        let active = self.handle.ok_or(ComputeApiError::InvalidHandle)?;
        if handle != active {
            if handle.kind() != BackendKind::Mock {
                return Err(ComputeApiError::ForeignHandle);
            }
            return Err(ComputeApiError::InvalidHandle);
        }
        if !self.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }
        validate_day_batch_cmd(&cmd)?;
        for count in cmd.output_spike_counts.iter_mut() {
            *count = 0;
        }
        Ok(BatchResult {
            ticks_executed: cmd.sync_batch_ticks,
            generated_spikes_count: 0,
            output_spikes_written: 0,
            dropped_spikes_count: 0,
            execution_time_us: 100,
        })
    }

    fn free_shard(&mut self, handle: VramHandle) -> Result<(), ComputeApiError> {
        if self.freed {
            return Err(ComputeApiError::AlreadyFreed);
        }
        let active = self.handle.ok_or(ComputeApiError::InvalidHandle)?;
        if handle != active {
            if handle.kind() != BackendKind::Mock {
                return Err(ComputeApiError::ForeignHandle);
            }
            return Err(ComputeApiError::InvalidHandle);
        }
        self.freed = true;
        self.handle = None;
        Ok(())
    }

    fn teardown(&mut self) -> Result<(), ComputeApiError> {
        self.handle = None;
        self.spec = None;
        self.uploaded = false;
        self.freed = false;
        Ok(())
    }
}

#[test]
fn test_trait_object_safety() {
    static_assertions::assert_obj_safe!(ComputeBackend);
    let mut mock = MockBackend::new();
    let trait_obj: &mut dyn ComputeBackend = &mut mock;
    assert_eq!(trait_obj.kind(), BackendKind::Mock);
}

#[test]
fn test_vram_handle_factory_and_accessors() {
    let id = NonZeroU64::new(100).unwrap();
    let handle = VramHandle::from_raw_parts(BackendKind::Cuda, id, 5);
    assert_eq!(handle.kind(), BackendKind::Cuda);
    assert_eq!(handle.id(), id);
    assert_eq!(handle.generation(), 5);
}

#[test]
fn test_reject_invalid_vram_handle() {
    let mut mock = MockBackend::new();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let _valid_handle = mock.alloc_shard(spec).unwrap();

    let invalid_handle =
        VramHandle::from_raw_parts(BackendKind::Mock, NonZeroU64::new(999).unwrap(), 1);
    let foreign_handle =
        VramHandle::from_raw_parts(BackendKind::Cpu, NonZeroU64::new(42).unwrap(), 1);

    let state_size = layout::calculate_state_blob_size(64);
    let state_buf = vec![0u8; state_size];
    let axons_buf = vec![0u8; 336]; // 16 + 10 * 32 = 336
    let dummy_variants = dummy_variants();
    let upload = ShardUpload {
        state_blob: &state_buf,
        axons_blob: &axons_buf,
        variant_table: &dummy_variants,
    };

    assert_eq!(
        mock.upload_shard(invalid_handle, upload),
        Err(ComputeApiError::InvalidHandle)
    );

    let upload2 = ShardUpload {
        state_blob: &state_buf,
        axons_blob: &axons_buf,
        variant_table: &dummy_variants,
    };
    assert_eq!(
        mock.upload_shard(foreign_handle, upload2),
        Err(ComputeApiError::ForeignHandle)
    );
}

#[test]
fn test_reject_freed_vram_handle() {
    let mut mock = MockBackend::new();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 0,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = mock.alloc_shard(spec).unwrap();
    assert!(mock.free_shard(handle).is_ok());
    assert_eq!(mock.free_shard(handle), Err(ComputeApiError::AlreadyFreed));
}

#[test]
fn test_reject_misaligned_padded_n() {
    let spec_zero = ShardAllocSpec {
        padded_n: 0,
        total_axons: 0,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    assert_eq!(
        validate_alloc_spec(&spec_zero),
        Err(ComputeApiError::InvalidShape)
    );

    let spec_unaligned = ShardAllocSpec {
        padded_n: 65,
        total_axons: 0,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    assert_eq!(
        validate_alloc_spec(&spec_unaligned),
        Err(ComputeApiError::AlignmentViolation)
    );

    let spec_valid = ShardAllocSpec {
        padded_n: 64,
        total_axons: 0,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    assert!(validate_alloc_spec(&spec_valid).is_ok());
}

#[test]
fn test_reject_invalid_v_seg() {
    let counts = [0u32; 1];
    let mut out_counts = [0u32; 1];
    let mut out_spikes = [0u32; 10];
    let soma_ids = [0u32; 1];

    let mut cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 0,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };

    assert_eq!(
        validate_day_batch_cmd(&cmd),
        Err(ComputeApiError::InvalidBatch)
    );

    cmd.v_seg = 256;
    assert_eq!(
        validate_day_batch_cmd(&cmd),
        Err(ComputeApiError::InvalidBatch)
    );

    cmd.v_seg = 1;
    assert!(validate_day_batch_cmd(&cmd).is_ok());
}

#[test]
fn test_reject_bad_state_blob_size() {
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 0,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let dummy_variants = dummy_variants();
    let bad_state = vec![0u8; 10];
    let axons = vec![0u8; 16];
    let upload = ShardUpload {
        state_blob: &bad_state,
        axons_blob: &axons,
        variant_table: &dummy_variants,
    };
    assert_eq!(
        validate_upload(&spec, &upload),
        Err(ComputeApiError::SizeMismatch)
    );
}

#[test]
fn test_validate_axons_blob_size_formula() {
    assert_eq!(expected_axons_blob_size(0).unwrap(), 16);
    assert_eq!(expected_axons_blob_size(1).unwrap(), 48);
    assert_eq!(expected_axons_blob_size(100).unwrap(), 3216);

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let dummy_variants = dummy_variants();
    let state_size = layout::calculate_state_blob_size(64);
    let state_buf = vec![0u8; state_size];
    let bad_axons_buf = vec![0u8; 100];
    let upload = ShardUpload {
        state_blob: &state_buf,
        axons_blob: &bad_axons_buf,
        variant_table: &dummy_variants,
    };
    assert_eq!(
        validate_upload(&spec, &upload),
        Err(ComputeApiError::SizeMismatch)
    );
}

#[test]
fn test_reject_insufficient_batch_slices() {
    let counts = [0u32; 1];
    let mut out_counts = [0u32; 1];
    let mut out_spikes = [0u32; 10];
    let soma_ids = [0u32; 1];

    // Wrong counts length
    let empty_counts: [u32; 0] = [];
    let cmd_wrong_counts = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &empty_counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };
    assert_eq!(
        validate_day_batch_cmd(&cmd_wrong_counts),
        Err(ComputeApiError::InvalidBatch)
    );

    // Output buffer too small
    let mut small_out_spikes = [0u32; 5];
    let cmd_small_out = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut small_out_spikes,
        output_spike_counts: &mut out_counts,
    };
    assert_eq!(
        validate_day_batch_cmd(&cmd_small_out),
        Err(ComputeApiError::CapacityExceeded)
    );

    // Incoming Some too short
    let short_inc_spikes = [0u32; 5];
    let cmd_short_inc = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: Some(&short_inc_spikes),
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };
    assert_eq!(
        validate_day_batch_cmd(&cmd_short_inc),
        Err(ComputeApiError::CapacityExceeded)
    );

    // Incoming None with nonzero counts
    let nonzero_counts = [5u32; 1];
    let cmd_nonzero_none = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &nonzero_counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };
    assert_eq!(
        validate_day_batch_cmd(&cmd_nonzero_none),
        Err(ComputeApiError::InvalidBatch)
    );
}

#[test]
fn test_validate_short_input_stride() {
    let counts = [0u32; 1];
    let mut out_counts = [0u32; 1];
    let mut out_spikes = [0u32; 10];
    let soma_ids = [0u32; 1];
    let input_bitmask = [0u32; 1];

    // For 33 axons, we need 2 words. We provide only 1 word, which should trigger InvalidBatch.
    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 33, // 33 axons -> needs 2 words per tick
        input_bitmask: Some(&input_bitmask),
        incoming_spikes: None,
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };
    assert_eq!(
        validate_day_batch_cmd(&cmd),
        Err(ComputeApiError::InvalidBatch)
    );

    // Exact count works (2 words for 33 axons)
    let input_bitmask_2 = [0u32; 2];
    let cmd_ok = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 2,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 33,
        input_bitmask: Some(&input_bitmask_2),
        incoming_spikes: None,
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };
    assert!(validate_day_batch_cmd(&cmd_ok).is_ok());
}

#[test]
fn test_default_debug_snapshot_returns_unsupported() {
    let mut mock = MockBackend::new();
    let handle = VramHandle::from_raw_parts(BackendKind::Mock, NonZeroU64::new(1).unwrap(), 1);
    let mut state = [0u8; 10];
    let mut axons = [0u8; 10];
    let snapshot = ShardSnapshotMut {
        state_blob: &mut state,
        axons_blob: &mut axons,
    };
    assert_eq!(
        mock.debug_snapshot(handle, snapshot),
        Err(ComputeApiError::UnsupportedFeature)
    );
}

#[test]
fn test_debug_snapshot_buffer_validation() {
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let state_size = layout::calculate_state_blob_size(64);
    let mut state_buf = vec![0u8; state_size];
    let mut axons_buf = vec![0u8; 336];

    let valid_snap = ShardSnapshotMut {
        state_blob: &mut state_buf,
        axons_blob: &mut axons_buf,
    };
    assert!(validate_snapshot_buffers(&spec, &valid_snap).is_ok());

    let mut bad_state = vec![0u8; 10];
    let bad_snap = ShardSnapshotMut {
        state_blob: &mut bad_state,
        axons_blob: &mut axons_buf,
    };
    assert_eq!(
        validate_snapshot_buffers(&spec, &bad_snap),
        Err(ComputeApiError::InvalidDebugProbeBounds)
    );
}

#[test]
fn test_api_returns_result_never_panics() {
    let spec_bad = ShardAllocSpec {
        padded_n: 13,
        total_axons: 0,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    assert!(validate_alloc_spec(&spec_bad).is_err());

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 0,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let dummy_variants = dummy_variants();
    let upload_bad = ShardUpload {
        state_blob: &[],
        axons_blob: &[],
        variant_table: &dummy_variants,
    };
    assert!(validate_upload(&spec, &upload_bad).is_err());
}

#[test]
fn test_mock_backend_implementation() {
    let mut mock = MockBackend::new();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = mock.alloc_shard(spec).unwrap();

    let dummy_variants = dummy_variants();
    let state_size = layout::calculate_state_blob_size(64);
    let state_buf = vec![0u8; state_size];
    let axons_buf = vec![0u8; 336];
    let upload = ShardUpload {
        state_blob: &state_buf,
        axons_blob: &axons_buf,
        variant_table: &dummy_variants,
    };
    mock.upload_shard(handle, upload).unwrap();

    let counts = [0u32; 2];
    let mut out_counts = [99u32; 2];
    let mut out_spikes = [0u32; 20];
    let soma_ids = [0u32; 1];

    let cmd = DayBatchCmd {
        tick_base: 100,
        sync_batch_ticks: 2,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };

    let res = mock.run_day_batch(handle, cmd).unwrap();
    assert_eq!(res.ticks_executed, 2);
    assert_eq!(out_counts, [0, 0]);
}

#[test]
fn test_validate_upload_rejects_invalid_alloc_spec() {
    let spec_zero = ShardAllocSpec {
        padded_n: 0,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let dummy_variants = dummy_variants();
    let upload = ShardUpload {
        state_blob: &[],
        axons_blob: &[],
        variant_table: &dummy_variants,
    };
    assert_eq!(
        validate_upload(&spec_zero, &upload),
        Err(ComputeApiError::InvalidShape)
    );

    let spec_misaligned = ShardAllocSpec {
        padded_n: 65,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    assert_eq!(
        validate_upload(&spec_misaligned, &upload),
        Err(ComputeApiError::AlignmentViolation)
    );
}

#[test]
fn test_validate_snapshot_buffers_rejects_invalid_alloc_spec() {
    let spec_zero = ShardAllocSpec {
        padded_n: 0,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let mut state_buf = [0u8; 0];
    let mut axons_buf = [0u8; 0];
    let snap = ShardSnapshotMut {
        state_blob: &mut state_buf,
        axons_blob: &mut axons_buf,
    };
    assert_eq!(
        validate_snapshot_buffers(&spec_zero, &snap),
        Err(ComputeApiError::InvalidShape)
    );

    let spec_misaligned = ShardAllocSpec {
        padded_n: 65,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let snap2 = ShardSnapshotMut {
        state_blob: &mut state_buf,
        axons_blob: &mut axons_buf,
    };
    assert_eq!(
        validate_snapshot_buffers(&spec_misaligned, &snap2),
        Err(ComputeApiError::AlignmentViolation)
    );
}

#[test]
fn test_no_vendor_feature_flags() {
    let cargo_toml = include_str!("../Cargo.toml");
    let mut in_features = false;
    let mut feature_keys = Vec::new();

    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
            continue;
        }
        if in_features && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some((key, _)) = trimmed.split_once('=') {
                feature_keys.push(key.trim());
            }
        }
    }

    feature_keys.sort();
    assert_eq!(feature_keys, vec!["default", "std"]);
}

#[test]
fn test_variant_table_lut_len() {
    let dummy_variants = dummy_variants();
    assert_eq!(dummy_variants.len(), 16);
    assert_eq!(layout::VARIANT_LUT_LEN, 16);
    let upload = ShardUpload {
        state_blob: &[],
        axons_blob: &[],
        variant_table: &dummy_variants,
    };
    assert_eq!(upload.variant_table.len(), layout::VARIANT_LUT_LEN);
}

#[test]
fn test_validate_day_batch_cmd_input_bitmask_validation() {
    let mut out_counts = [0u32; 1];
    let mut out_spikes = [0u32; 10];
    let soma_ids = [0u32; 1];
    let counts = [0u32; 1];
    let bitmask = [0u32; 0];

    // Invalid Case: input_bitmask is Some, num_virtual_axons = 32 (> 0) but input_words_per_tick = 0
    let cmd_invalid = DayBatchCmd {
        tick_base: 100,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 32,
        input_bitmask: Some(&bitmask),
        incoming_spikes: None,
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };

    assert_eq!(
        validation::validate_day_batch_cmd(&cmd_invalid),
        Err(ComputeApiError::InvalidBatch)
    );

    // Valid Case: input_bitmask is Some, num_virtual_axons = 32, input_words_per_tick = 1
    let mut out_counts_v = [0u32; 1];
    let mut out_spikes_v = [0u32; 10];
    let bitmask_valid = [0u32; 1];
    let cmd_valid = DayBatchCmd {
        tick_base: 100,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 32,
        input_bitmask: Some(&bitmask_valid),
        incoming_spikes: None,
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes_v,
        output_spike_counts: &mut out_counts_v,
    };

    assert!(validation::validate_day_batch_cmd(&cmd_valid).is_ok());
}

#[test]
fn test_validate_day_batch_cmd_tick_overflow() {
    let mut out_counts = [0u32; 2];
    let mut out_spikes = [0u32; 20];
    let soma_ids = [0u32; 1];
    let counts = [0u32; 2];

    // Case 1: tick_base = u64::MAX, sync_batch_ticks = 2 -> tick_base + sync_batch_ticks - 1 overflows u64
    let cmd_overflow = DayBatchCmd {
        tick_base: u64::MAX,
        sync_batch_ticks: 2,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 10,
        num_outputs: 1,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &counts,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };

    assert_eq!(
        validation::validate_day_batch_cmd(&cmd_overflow),
        Err(ComputeApiError::InvalidBatch)
    );

    // Case 2: tick_base = u64::MAX, sync_batch_ticks = 1 -> tick_base + sync_batch_ticks - 1 does not overflow
    let mut out_counts_ok = [0u32; 1];
    let mut out_spikes_ok = [0u32; 10];
    let counts_ok = [0u32; 1];
    let cmd_ok = DayBatchCmd {
        tick_base: u64::MAX,
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
        incoming_spike_counts: &counts_ok,
        mapped_soma_ids: &soma_ids,
        output_spikes: &mut out_spikes_ok,
        output_spike_counts: &mut out_counts_ok,
    };

    assert!(validation::validate_day_batch_cmd(&cmd_ok).is_ok());
}
