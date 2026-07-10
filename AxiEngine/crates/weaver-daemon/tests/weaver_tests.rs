use weaver_daemon::*;

#[test]
fn test_host_slices_pipeline_run() {
    let padded_n = 64;
    let total_axons = 100;
    let total_ghosts = 10;

    // Create a mock segment to allocate memory mapped view safely
    let mut segment = ipc::ShmSegment::create_mock(0x11223344, padded_n, total_axons, total_ghosts)
        .expect("Failed to create mock SHM segment");

    {
        let view = segment.as_working_view_mut();
        let off_targets = view.offsets.off_targets;
        let off_weights = view.offsets.off_weights;
        let targets_slice =
            bytemuck::cast_slice_mut::<u8, u32>(&mut view.state_blob[off_targets..off_weights]);
        for target in targets_slice.iter_mut() {
            *target = types::EMPTY_PIXEL;
        }
    }

    let view = segment.as_working_view_mut();
    let mut source = NightBufferSource::HostSlices(view);

    let req = WeaverJobRequest {
        shard_id: 1,
        zone_hash: 0x11223344,
        night_epoch: 5,
        master_seed: [1u8; 32],
        prune_threshold: 10,
        max_sprouts: 5,
        w_distance: 1,
        w_power: 2,
        w_explore: 3,
        initial_synapse_weight: 100,
        has_growth_context: false,
    };

    let (report, handovers) =
        run_night_pipeline(&req, None, &mut source).expect("Failed to run pipeline");

    assert_eq!(report.shard_id, 1);
    assert_eq!(report.night_epoch, 5);
    assert_eq!(report.sprouted_count, 5); // Sprouted exactly max_sprouts
    assert_eq!(handovers.len(), 0);
}

#[test]
fn test_pipeline_real_apply_pruning() {
    let padded_n = 64;
    let total_axons = 100;
    let total_ghosts = 10;

    let mut segment = ipc::ShmSegment::create_mock(0x11223344, padded_n, total_axons, total_ghosts)
        .expect("Failed to create mock SHM segment");

    {
        let view = segment.as_working_view_mut();
        // Populate one active synapse that will be pruned
        let off_targets = view.offsets.off_targets;
        let off_weights = view.offsets.off_weights;
        let off_dtimers = view.offsets.off_dtimers;

        let state_bytes = view.state_blob;
        let (_, rest) = state_bytes.split_at_mut(off_targets);
        let (targets_bytes, rest) = rest.split_at_mut(off_weights - off_targets);
        let (weights_bytes, _) = rest.split_at_mut(off_dtimers - off_weights);

        let targets_slice = bytemuck::cast_slice_mut::<u8, u32>(targets_bytes);
        let weights_slice = bytemuck::cast_slice_mut::<u8, i32>(weights_bytes);

        for target in targets_slice.iter_mut() {
            *target = types::EMPTY_PIXEL;
        }

        // Slot 0 of soma 0: active target but weight is 5 (which is below threshold 10)
        let idx = 0;
        targets_slice[idx] = 100u32;
        weights_slice[idx] = 5;
    }

    let view = segment.as_working_view_mut();
    let mut source = NightBufferSource::HostSlices(view);

    let req = WeaverJobRequest {
        shard_id: 1,
        zone_hash: 0x11223344,
        night_epoch: 5,
        master_seed: [1u8; 32],
        prune_threshold: 10, // pruning threshold
        max_sprouts: 0,      // no sprouting to isolate pruning
        w_distance: 1,
        w_power: 2,
        w_explore: 3,
        initial_synapse_weight: 100,
        has_growth_context: false,
    };

    let (report, _) = run_night_pipeline(&req, None, &mut source).expect("Failed to run pipeline");

    assert_eq!(report.pruned_count, 1);

    // Verify that the slot was indeed pruned in the storage
    let view_after = segment.as_working_view_mut();
    let targets_slice = bytemuck::cast_slice::<u8, u32>(
        &view_after.state_blob[view_after.offsets.off_targets..view_after.offsets.off_weights],
    );
    assert_eq!(targets_slice[0], types::EMPTY_PIXEL);
}

#[test]
fn test_no_compute_dependencies() {
    let cargo_toml_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let content = std::fs::read_to_string(cargo_toml_path).expect("Failed to read Cargo.toml");

    let forbidden = ["compute", "compute-api", "compute-cpu", "compute-cuda"];
    for line in content.lines() {
        let line_trimmed = line.trim();
        if !line_trimmed.starts_with('#') {
            for &f in &forbidden {
                if line_trimmed.contains(f)
                    && (line_trimmed.contains("path = ") || line_trimmed.contains("version = "))
                {
                    panic!("Forbidden dependency '{}' found in Cargo.toml!", f);
                }
            }
        }
    }
}

#[test]
fn test_pipeline_negative_weight_pruning() {
    let padded_n = 64;
    let total_axons = 100;
    let total_ghosts = 10;

    let mut segment = ipc::ShmSegment::create_mock(0x11223344, padded_n, total_axons, total_ghosts)
        .expect("Failed to create mock SHM segment");

    {
        let view = segment.as_working_view_mut();
        let off_targets = view.offsets.off_targets;
        let off_weights = view.offsets.off_weights;
        let off_dtimers = view.offsets.off_dtimers;

        let state_bytes = view.state_blob;
        let (_, rest) = state_bytes.split_at_mut(off_targets);
        let (targets_bytes, rest) = rest.split_at_mut(off_weights - off_targets);
        let (weights_bytes, _) = rest.split_at_mut(off_dtimers - off_weights);

        let targets_slice = bytemuck::cast_slice_mut::<u8, types::PackedTarget>(targets_bytes);
        let weights_slice = bytemuck::cast_slice_mut::<u8, i32>(weights_bytes);

        for target in targets_slice.iter_mut() {
            *target = types::PackedTarget::TOMBSTONE;
        }

        // Soma 0, slot 0: weight -3 (pruned because |-3| = 3 < 5)
        targets_slice[0] = types::PackedTarget::pack(1, 0);
        weights_slice[0] = -3;

        // Soma 0, slot 1: weight -10 (NOT pruned because |-10| = 10 >= 5)
        let p_n = padded_n as usize;
        targets_slice[p_n] = types::PackedTarget::pack(2, 0);
        weights_slice[p_n] = -10;
    }

    let view = segment.as_working_view_mut();
    let mut source = NightBufferSource::HostSlices(view);

    let req = WeaverJobRequest {
        shard_id: 1,
        zone_hash: 0x11223344,
        night_epoch: 5,
        master_seed: [1u8; 32],
        prune_threshold: 5,
        max_sprouts: 0,
        w_distance: 1,
        w_power: 2,
        w_explore: 3,
        initial_synapse_weight: 100,
        has_growth_context: false,
    };

    let (report, _) = run_night_pipeline(&req, None, &mut source).expect("Failed to run pipeline");

    assert_eq!(report.pruned_count, 1);

    let view_after = segment.as_working_view_mut();
    let targets_slice = bytemuck::cast_slice::<u8, types::PackedTarget>(
        &view_after.state_blob[view_after.offsets.off_targets..view_after.offsets.off_weights],
    );
    let p_n = padded_n as usize;
    assert_eq!(targets_slice[0], types::PackedTarget::pack(2, 0));
    assert_eq!(targets_slice[p_n], types::PackedTarget::TOMBSTONE);
}

#[test]
fn test_pipeline_compaction_after_prune() {
    let padded_n = 64;
    let total_axons = 100;
    let total_ghosts = 10;

    let mut segment = ipc::ShmSegment::create_mock(0x11223344, padded_n, total_axons, total_ghosts)
        .expect("Failed to create mock SHM segment");

    {
        let view = segment.as_working_view_mut();
        let off_targets = view.offsets.off_targets;
        let off_weights = view.offsets.off_weights;
        let off_dtimers = view.offsets.off_dtimers;

        let state_bytes = view.state_blob;
        let (_, rest) = state_bytes.split_at_mut(off_targets);
        let (targets_bytes, rest) = rest.split_at_mut(off_weights - off_targets);
        let (weights_bytes, _) = rest.split_at_mut(off_dtimers - off_weights);

        let targets_slice = bytemuck::cast_slice_mut::<u8, types::PackedTarget>(targets_bytes);
        let weights_slice = bytemuck::cast_slice_mut::<u8, i32>(weights_bytes);

        for target in targets_slice.iter_mut() {
            *target = types::PackedTarget::TOMBSTONE;
        }

        let p_n = padded_n as usize;
        // Setup for Soma 0:
        // slot 0: active target, weight 20
        targets_slice[0] = types::PackedTarget::pack(1, 0);
        weights_slice[0] = 20;

        // slot 1: active target, weight 2 (will be pruned by threshold 10)
        targets_slice[p_n] = types::PackedTarget::pack(2, 0);
        weights_slice[p_n] = 2;

        // slot 2: active target, weight 30
        targets_slice[p_n * 2] = types::PackedTarget::pack(3, 0);
        weights_slice[p_n * 2] = 30;

        // slot 3: inactive target (0)
        targets_slice[p_n * 3] = types::PackedTarget::NONE;
        weights_slice[p_n * 3] = 0;

        // slot 4: active target, weight 40
        targets_slice[p_n * 4] = types::PackedTarget::pack(4, 0);
        weights_slice[p_n * 4] = 40;
    }

    let view = segment.as_working_view_mut();
    let mut source = NightBufferSource::HostSlices(view);

    let req = WeaverJobRequest {
        shard_id: 1,
        zone_hash: 0x11223344,
        night_epoch: 5,
        master_seed: [1u8; 32],
        prune_threshold: 10,
        max_sprouts: 0,
        w_distance: 1,
        w_power: 2,
        w_explore: 3,
        initial_synapse_weight: 100,
        has_growth_context: false,
    };

    let (report, _) = run_night_pipeline(&req, None, &mut source).expect("Failed to run pipeline");

    // Pruned slot 1 (weight 2 < 10)
    assert_eq!(report.pruned_count, 1);

    // Compaction should shift active targets to the front (indices 0, 1, 2).
    // Original active targets remaining:
    // target(1, 0) at index 0 (slot 0)
    // target(3, 0) at index 2 -> moves to slot 1
    // target(4, 0) at index 4 -> moves to slot 2
    // Report compaction moves: slot 2 -> 1, slot 4 -> 2, which is 2 moves.
    assert_eq!(report.compacted_count, 2);

    let view_after = segment.as_working_view_mut();
    let targets_slice = bytemuck::cast_slice::<u8, types::PackedTarget>(
        &view_after.state_blob[view_after.offsets.off_targets..view_after.offsets.off_weights],
    );

    let p_n = padded_n as usize;
    assert_eq!(targets_slice[0], types::PackedTarget::pack(1, 0));
    assert_eq!(targets_slice[p_n], types::PackedTarget::pack(3, 0));
    assert_eq!(targets_slice[p_n * 2], types::PackedTarget::pack(4, 0));
    // The rest must be tombstoned
    assert_eq!(targets_slice[p_n * 3], types::PackedTarget::TOMBSTONE);
    assert_eq!(targets_slice[p_n * 4], types::PackedTarget::TOMBSTONE);
}
