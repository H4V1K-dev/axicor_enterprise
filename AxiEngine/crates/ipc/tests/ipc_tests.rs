use ipc::*;
use std::time::Duration;

#[test]
fn test_deterministic_shm_names() {
    let zone_hash = 0x12345678;
    assert_eq!(shm_segment_name(zone_hash), "axicor_shard_12345678");
    assert_eq!(
        manifest_file_name(zone_hash),
        "axicor_manifest_12345678.toml"
    );
    assert_eq!(ephys_segment_name(zone_hash), "axicor_ephys_12345678.shm");
}

#[test]
fn test_windows_named_pipe_names() {
    let zone_hash = 0xABCDEF01;
    let expected = r"\\.\pipe\axicor_baker_ABCDEF01";
    assert_eq!(format_windows_pipe(zone_hash), expected);
    assert_eq!(
        format_linux_uds(zone_hash, Some("/run/user/1000"), 1000),
        "/run/user/1000/axiengine/axicor_baker_ABCDEF01.sock"
    );
    assert_eq!(
        format_linux_uds(zone_hash, None, 1000),
        "/tmp/axiengine-1000/axicor_baker_ABCDEF01.sock"
    );
}

#[test]
fn test_cold_start_evicts_stale() {
    let zone_hash = 0xDEADE1CC;
    // Create one segment (exclusive owner)
    let segment1 = ShmSegment::create(zone_hash, 64, 100, 10).expect("Failed to create SHM");
    assert_eq!(segment1.header().zone_hash, zone_hash);

    // Creating another one should evict and recreate successfully
    let segment2 = ShmSegment::create(zone_hash, 64, 100, 10).expect("Failed to recreate SHM");
    assert_eq!(segment2.header().zone_hash, zone_hash);
}

#[test]
fn test_attach_rejects_bad_magic_and_version() {
    let zone_hash = 0xBAD11111;
    {
        let mut segment = ShmSegment::create(zone_hash, 64, 100, 10).expect("Failed to create SHM");

        // Corrupt magic
        segment.header_mut().magic = *b"CORR";
    }

    // Try to attach, should fail because magic is corrupted
    let attach_res = ShmSegment::attach(zone_hash);
    assert!(matches!(attach_res, Err(IpcError::PoisonedSegment)));

    // Recreate clean
    {
        let mut segment =
            ShmSegment::create(zone_hash, 64, 100, 10).expect("Failed to recreate SHM");

        // Corrupt version
        segment.header_mut().version = 2;
    }

    // Try to attach, should fail because version is mismatched
    let attach_res2 = ShmSegment::attach(zone_hash);
    assert!(matches!(attach_res2, Err(IpcError::PoisonedSegment)));
}

#[test]
fn test_state_machine_transitions() {
    let zone_hash = 0x5555AAAA;
    let mut segment = ShmSegment::create(zone_hash, 64, 100, 10).expect("Failed to create SHM");
    assert_eq!(segment.get_state(), NightState::Idle);

    // Idle -> NightStart (Valid)
    assert!(segment
        .try_transition(NightState::Idle, NightState::NightStart)
        .is_ok());
    assert_eq!(segment.get_state(), NightState::NightStart);

    // NightStart -> Sprouting (Valid)
    assert!(segment
        .try_transition(NightState::NightStart, NightState::Sprouting)
        .is_ok());
    assert_eq!(segment.get_state(), NightState::Sprouting);

    // Sprouting -> NightStart (Invalid)
    assert!(segment
        .try_transition(NightState::Sprouting, NightState::NightStart)
        .is_err());

    // Sprouting -> NightDone (Valid)
    assert!(segment
        .try_transition(NightState::Sprouting, NightState::NightDone)
        .is_ok());
    assert_eq!(segment.get_state(), NightState::NightDone);

    // NightDone -> Idle (Valid)
    assert!(segment
        .try_transition(NightState::NightDone, NightState::Idle)
        .is_ok());
    assert_eq!(segment.get_state(), NightState::Idle);

    // Idle -> Error (Valid)
    assert!(segment
        .try_transition(NightState::Idle, NightState::Error)
        .is_ok());
    assert_eq!(segment.get_state(), NightState::Error);

    // Error -> Idle (Valid for cold path reset)
    assert!(segment
        .try_transition(NightState::Error, NightState::Idle)
        .is_ok());
    assert_eq!(segment.get_state(), NightState::Idle);
}

#[test]
fn test_state_machine_timeout_to_error() {
    let zone_hash = 0x7777BBBB;
    let mut segment = ShmSegment::create(zone_hash, 64, 100, 10).expect("Failed to create SHM");
    assert_eq!(segment.get_state(), NightState::Idle);

    segment
        .try_transition(NightState::Idle, NightState::NightStart)
        .unwrap();
    segment
        .try_transition(NightState::NightStart, NightState::Sprouting)
        .unwrap();

    // Try waiting for NightDone but daemon is hanging. Timeout should move it to Error state.
    let wait_res = segment.wait_for_state(NightState::NightDone, Duration::from_millis(10));
    assert!(matches!(wait_res, Err(IpcError::Timeout)));
    assert_eq!(segment.get_state(), NightState::Error);
}

#[test]
fn test_swapchain_publish_consume_visibility() {
    let capacity = 1024;
    let swapchain = InputSwapchain::new(capacity);

    // Initially back buffer can be written to
    swapchain.write_incoming_at(0, &[10, 20, 30]).unwrap();
    swapchain.write_incoming_at(100, &[40, 50]).unwrap();

    // Verify ready buffer is not updated yet (all zeroes)
    let ready_ptr = swapchain.consume_for_gpu();
    unsafe {
        assert_eq!(*ready_ptr.add(0), 0);
        assert_eq!(*ready_ptr.add(100), 0);
    }

    // Swap buffers
    swapchain.swap();

    // Verify ready buffer now contains the data
    let ready_ptr2 = swapchain.consume_for_gpu();
    unsafe {
        assert_eq!(*ready_ptr2.add(0), 10);
        assert_eq!(*ready_ptr2.add(1), 20);
        assert_eq!(*ready_ptr2.add(2), 30);
        assert_eq!(*ready_ptr2.add(100), 40);
        assert_eq!(*ready_ptr2.add(101), 50);
    }

    // Back buffer now contains old ready buffer (which is clean). Write and swap again.
    swapchain.write_incoming_at(0, &[9, 8]).unwrap();
    swapchain.swap();

    let ready_ptr3 = swapchain.consume_for_gpu();
    unsafe {
        assert_eq!(*ready_ptr3.add(0), 9);
        assert_eq!(*ready_ptr3.add(1), 8);
        assert_eq!(*ready_ptr3.add(2), 0); // cleared or unchanged because it was 0 in ready
    }
}

#[test]
fn test_swapchain_capacity_overflow() {
    let capacity = 10;
    let swapchain = InputSwapchain::new(capacity);

    // Exact limit
    assert!(swapchain.write_incoming_at(0, &[1; 10]).is_ok());

    // Exceed capacity
    assert!(matches!(
        swapchain.write_incoming_at(0, &[1; 11]),
        Err(IpcError::CapacityExceeded)
    ));

    // Exceed capacity via offset
    assert!(matches!(
        swapchain.write_incoming_at(5, &[1; 6]),
        Err(IpcError::CapacityExceeded)
    ));
}

#[test]
fn test_mock_shm_allocator_isolation() {
    let zone_hash = 0x11119999;
    let padded_n = 128;
    let total_axons = 50;
    let total_ghosts = 5;

    let mut segment = MockShmAllocator::allocate(zone_hash, padded_n, total_axons, total_ghosts)
        .expect("Mock allocation failed");

    // Header values
    let header = segment.header();
    assert_eq!(header.magic, *b"AXSM");
    assert_eq!(header.padded_n, padded_n);
    assert_eq!(header.total_axons, total_axons);
    assert_eq!(header.zone_hash, zone_hash);

    // Working views validation
    let view = segment.as_working_view_mut();
    assert_eq!(view.padded_n, padded_n);
    assert_eq!(view.total_axons, total_axons);
    assert_eq!(view.total_ghosts, total_ghosts);
}

#[test]
fn test_weaver_job_dto_serialization() {
    let job = WeaverJobRequest {
        shard_id: 42,
        zone_hash: 0x99887766,
        night_epoch: 1234,
        master_seed: [7u8; 32],
        prune_threshold: 15,
        max_sprouts: 200,
        w_distance: 100,
        w_power: 200,
        w_explore: 300,
        initial_synapse_weight: -10,
        has_growth_context: true,
    };

    let serialized = serde_json::to_string(&job).expect("Failed to serialize DTO");
    let deserialized: WeaverJobRequest =
        serde_json::from_str(&serialized).expect("Failed to deserialize DTO");

    assert_eq!(deserialized.shard_id, job.shard_id);
    assert_eq!(deserialized.zone_hash, job.zone_hash);
    assert_eq!(deserialized.night_epoch, job.night_epoch);
    assert_eq!(deserialized.master_seed, job.master_seed);
    assert_eq!(deserialized.prune_threshold, job.prune_threshold);
    assert_eq!(deserialized.max_sprouts, job.max_sprouts);
    assert_eq!(
        deserialized.initial_synapse_weight,
        job.initial_synapse_weight
    );
    assert_eq!(deserialized.has_growth_context, job.has_growth_context);
}
