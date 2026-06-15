//! weaver-daemon — Night Phase orchestration binary.
//!
//! # INV-WDAEMON-001: Zero GPU Knowledge
//! This binary is strictly GPU-blind. It must never link to compute-api,
//! compute-cuda, compute-hip, or any GPU driver crate. All GPU interaction
//! is mediated exclusively through the shared memory IPC protocol.
//!
//! # Fail-Fast Contract
//! Any OS-level failure (socket bind error, config read error, SHM map error,
//! or IPC response error) terminates the process via `std::process::exit(1)`.
//! `unwrap()` on fallible operations is strictly forbidden.

pub mod compaction;
pub mod worker;

/// Command-line arguments for weaver-daemon.
///
/// Configured at process start by the OS orchestrator.
#[derive(clap::Parser, Debug)]
struct Cli {
    #[arg(short, long)]
    config_dir: std::path::PathBuf,
    #[arg(short, long)]
    zone_hash: u32,
    #[arg(long, default_value_t = false)]
    mock: bool,
}

fn main() {
    // ── Phase 1: Initialize tracing subscriber ─────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // ── Phase 2: Parse CLI arguments (clap) ───────────────────────────────────
    use clap::Parser as _;
    let cli = Cli::parse();
    tracing::info!(
        zone_hash = cli.zone_hash,
        config_dir = %cli.config_dir.display(),
        mock = cli.mock,
        "weaver-daemon starting"
    );

    // ── Phase 3: Load blueprints configuration ────────────────────────────────
    let blueprints_path = cli.config_dir.join("blueprints.toml");
    let blueprints_content = match std::fs::read_to_string(&blueprints_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(path = %blueprints_path.display(), error = %e, "Failed to read blueprints.toml");
            std::process::exit(1);
        }
    };
    let blueprints = match config::parse_blueprints_config(&blueprints_content) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "Failed to parse blueprints.toml");
            std::process::exit(1);
        }
    };
    tracing::info!(
        neuron_types = blueprints.neuron_types.len(),
        "Blueprints loaded"
    );

    // ── Phase 4: Mock path — autonomous CI smoke test ─────────────────────────
    //
    // When --mock is set the daemon must NOT open any socket. It allocates a
    // fresh in-process SHM region, runs one Night Phase cycle, then exits(0).
    // This allows the CI pipeline to verify correctness without a live GPU node.
    if cli.mock {
        tracing::info!(zone_hash = cli.zone_hash, "Mock mode: running single cycle and exiting");

        let mut shm_manager = ipc::ShmManager::create_cold(cli.zone_hash, 128).unwrap_or_else(|_| std::process::exit(1));

        let shm_ptr = shm_manager.mapped.mmap.as_mut_ptr();

        let hdr = ipc::validate_shm_header(shm_ptr).unwrap_or_else(|_| std::process::exit(1));

        let padded_n = hdr.padded_n as usize;
        let soma_positions: Vec<types::PackedPosition> = (0..padded_n)
            .map(|_| types::PackedPosition::pack_raw(0, 0, 0, 0))
            .collect();

        let mut ctx = worker::DaemonContext {
            living_axons: Vec::new(),
        };

        worker::process_night(
            &mut ctx,
            shm_ptr,
            hdr,
            &blueprints,
            (1000, 1000, 255),
            10,    // prune_threshold placeholder
            1000,  // max_sprouts placeholder
            &soma_positions,
        ).unwrap_or_else(|_| std::process::exit(1));

        tracing::info!("Mock: Night Phase complete — OK");
        std::process::exit(0);
    }

    // ── Phase 5: Bind IPC server ───────────────────────────────────────────────
    // Fail-Fast: if the socket is already in use or OS denies bind, exit(1).
    let server = match ipc::BakerServer::bind(cli.zone_hash) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(zone_hash = cli.zone_hash, error = ?e, "Failed to bind IPC socket");
            std::process::exit(1);
        }
    };
    tracing::info!(zone_hash = cli.zone_hash, "IPC server bound, entering event loop");

    // ── Phase 6: Initialize daemon context ────────────────────────────────────
    // DaemonContext holds CPU-side state of growing axon tips (Night Phase only).
    // Starts empty; the GPU node populates living axons via AxonHandoverEvents.
    let mut ctx = worker::DaemonContext {
        living_axons: Vec::new(),
    };

    // ── Phase 7: Night Phase IPC event loop ───────────────────────────────────
    //
    // Each iteration:
    //   1. Accept a connection from the GPU node (axicor-node).
    //   2. Receive BakeRequest + AxonHandoverEvents.
    //   3. Attach + validate shared memory (C-ABI contract).
    //   4. INV-WDAEMON-005: State Safety Lock — verify SHM is in Sprouting.
    //   5. Execute the Night Phase pipeline (worker::process_night).
    //   6. INV-CROSS-013: Transition SHM → NightDone to unblock the GPU node.
    //   7. Send AxonHandoverAcks back.
    loop {
        // 7.1: Accept incoming connection from the GPU node.
        let mut conn = match server.accept() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to accept IPC connection, retrying");
                // Non-fatal: accept can transiently fail; keep looping.
                continue;
            }
        };
        tracing::debug!("IPC connection accepted");

        // 7.2: Receive BakeRequest and axon handover events.
        let (req, handovers) = match conn.recv_request() {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to receive BakeRequest");
                // Connection-level error: the GPU node may have crashed. Skip.
                continue;
            }
        };
        tracing::info!(
            zone_hash = req.zone_hash,
            current_tick = req.current_tick,
            prune_threshold = req.prune_threshold,
            max_sprouts = req.max_sprouts,
            handovers = handovers.len(),
            "Night Phase request received"
        );

        // 7.3: Integrate incoming AxonHandoverEvents into daemon context.
        //
        // Each handover event represents an axon tip entering this zone from
        // a neighbouring zone. We instantiate a LivingAxon from the event data.
        for handover in &handovers {
            ctx.living_axons.push(topology::types::LivingAxon {
                axon_id: handover.local_axon_id as usize,
                soma_idx: 0, // resolved via SHM state arrays in future phase
                tip_uvw: types::PackedPosition::pack_raw(
                    handover.entry_x as u32,
                    handover.entry_y as u32,
                    handover.entry_z as u32,
                    0,
                )
                .0,
                forward_dir: glam::Vec3::new(
                    handover.vector_x as f32,
                    handover.vector_y as f32,
                    handover.vector_z as f32,
                ),
                remaining_steps: handover.remaining_length as u32,
                last_night_active: false,
            });
        }

        // 7.4: Attach and validate shared memory (production path only).
        let shm_path = ipc::utils::shm_file_path(req.zone_hash);
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&shm_path)
        {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(path = %shm_path.display(), error = %e, "Failed to open SHM file");
                std::process::exit(1);
            }
        };
        let mapped = match ipc::MappedShm::new(&file, 0) {
            Ok(m) => m,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to mmap SHM file");
                std::process::exit(1);
            }
        };
        let mut shm_manager = ipc::ShmManager { mapped };
        let shm_ptr = shm_manager.mapped.mmap.as_mut_ptr();

        // Validate C-ABI header (INV-CROSS-008 / E-030).
        let hdr = match ipc::validate_shm_header(shm_ptr) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!(error = ?e, "SHM header validation failed (C-ABI contract violated)");
                // INV-WDAEMON-005: Force Error state so the GPU node can recover.
                let state_ptr = unsafe { &*(std::ptr::addr_of!((*shm_ptr.cast::<layout::ShmHeader>()).state) as *const std::sync::atomic::AtomicU8) };
                let sm = unsafe { ipc::ShmStateMachine::new(state_ptr) };
                sm.mark_error();
                continue;
            }
        };

        // 7.5: INV-WDAEMON-005 State Safety Lock.
        //
        // Verify the orchestrator has set SHM to Sprouting before we write.
        // If the state is anything else (e.g. the orchestrator timed-out and
        // reset to Error), writing would race with the HFT cycle → Data Race.
        let state_ptr = unsafe { &*(std::ptr::addr_of!((*shm_ptr.cast::<layout::ShmHeader>()).state) as *const std::sync::atomic::AtomicU8) };
        let sm = unsafe { ipc::ShmStateMachine::new(state_ptr) };
        if state_ptr.load(std::sync::atomic::Ordering::Acquire) != layout::ShmState::Sprouting as u8 {
            continue;
        }

        // 7.6: Build soma_positions stub (placeholder for shard.toml integration).
        //
        // TODO: Pull real soma positions from SHM state arrays when shard.toml
        // layout binding is available. Using stub (1000, 1000, 255) bounds for now.
        let padded_n = hdr.padded_n as usize;
        let soma_positions: Vec<types::PackedPosition> = (0..padded_n)
            .map(|_| types::PackedPosition::pack_raw(0, 0, 0, 0))
            .collect();

        // 7.7: Execute the Night Phase pipeline.
        //
        // INV-WDAEMON-005: on pipeline error, force SHM to Error state.
        let bounds = (1000u32, 1000u32, 255u32);
        let result = worker::process_night(
            &mut ctx,
            shm_ptr,
            hdr,
            &blueprints,
            bounds,
            req.prune_threshold,
            req.max_sprouts as u32,
            &soma_positions,
        );

        if let Err(e) = result {
            tracing::error!(error = %e, "Night Phase pipeline failed");
            sm.mark_error();
            continue;
        }

        tracing::info!(
            zone_hash = req.zone_hash,
            current_tick = req.current_tick,
            "Night Phase pipeline complete"
        );

        // 7.8: INV-CROSS-013: Transition SHM → NightDone.
        //
        // This unblocks the GPU node (axicor-node) which is spin-waiting on
        // ShmState::NightDone. Failing to do this would leave it hanging forever.
        // Fail-Fast: a CAS failure here means another writer corrupted the state —
        // this is unrecoverable; exit(1) to trigger cluster-level resurrection.
        sm.transition(layout::ShmState::Sprouting as u8, layout::ShmState::NightDone as u8).unwrap_or_else(|_| std::process::exit(1));

        // 7.9: Send AxonHandoverAcks back to the GPU node.
        //
        // Currently no axon handovers are generated by weaver-daemon itself
        // (ghost handover queue integration is a future phase). We respond
        // with an empty ack list to unblock the GPU node.
        if let Err(e) = conn.send_response(&[]) {
            tracing::error!(error = ?e, "Failed to send Night Phase response");
            // Non-fatal for the daemon; the GPU node will handle the timeout.
        }
    }
}
