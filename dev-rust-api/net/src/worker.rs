use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

/// High-Frequency Trading (HFT) network worker.
///
/// # Zero-Alloc Hot Path Invariant (INV-NET-002)
/// Under INV-NET-002, the processing of inbound spike packets and dispatching of outbound
/// spike batches must not trigger dynamic heap allocations (such as `Vec::new()`, `Box::new()`,
/// or resizing collections) in the data plane to avoid CPU cache thrashing and memory latency spikes.
///
/// # Slow/Fast Path Asymmetry Invariant (INV-NET-005)
/// Under INV-NET-005, the UDP fast path runs in a dedicated thread and does not block or share
/// resources with the TCP geometry sync or telemetry WebSocket server to prevent Head-of-Line blocking.
pub struct UdpWorker {
    pub socket: transport::FastPathSocket,
    pub routes: Arc<crate::routing::RoutingTable>,
    pub egress_pool: Arc<transport::EgressPool>,
    pub reassembly: protocol::ReassemblyBuffer,
    pub current_epoch: Arc<AtomicU32>,
    /// Shutdown flag to break the infinite loop gracefully.
    pub shutdown: Arc<AtomicBool>,
}

impl UdpWorker {
    /// Create a new UdpWorker.
    pub fn new(
        socket: transport::FastPathSocket,
        routes: Arc<crate::routing::RoutingTable>,
        egress_pool: Arc<transport::EgressPool>,
        reassembly: protocol::ReassemblyBuffer,
        current_epoch: Arc<AtomicU32>,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            socket,
            routes,
            egress_pool,
            reassembly,
            current_epoch,
            shutdown,
        }
    }

    /// Run the infinite HFT data plane processing loop.
    ///
    /// Inside this function, heap allocation (system allocator) is strictly prohibited.
    pub fn run(&mut self) {
        let mut recv_buf = [0u8; 65536];
        let mut send_buf = [0u8; 1500]; // Standard MTU size for stack serialization

        while !self.shutdown.load(Ordering::Relaxed) {
            // Inbound Phase (Receiving spike batches)
            match self.socket.recv_from(&mut recv_buf) {
                Ok((len, _src_addr)) => {
                    // Decode header and spikes from packet
                    match protocol::decode_spike_batch(&recv_buf[..len]) {
                        Ok((header, _spikes)) => {
                            let node_epoch = self.current_epoch.load(Ordering::Acquire);
                            
                            // Biological Amnesia Pattern (E-124, INV-CROSS-012)
                            let epoch_verdict = protocol::validate_epoch_math(
                                header.epoch,
                                node_epoch,
                                protocol::DEFAULT_TOLERANCE,
                                protocol::DEFAULT_SELF_HEALING_THRESHOLD,
                            );

                            match epoch_verdict {
                                protocol::EpochAction::AmnesiaDrop => {
                                    // Drop outdated packets immediately to preserve causality
                                    continue;
                                }
                                protocol::EpochAction::SelfHealingFastForward(target_epoch) => {
                                    // Node is lagging, force fast-forward local epoch
                                    self.current_epoch.store(target_epoch, Ordering::Release);
                                }
                                protocol::EpochAction::Accept => {}
                            }

                            // Feed the chunk into the reassembly buffer
                            let _ = self.reassembly.insert_chunk(&header, &recv_buf[16..len]);
                        }
                        Err(_) => {
                            // Invalid packet size/alignment, discard silently in HFT loop
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No packets to read, which is expected for non-blocking I/O
                }
                Err(_) => {
                    // System I/O error, ignore or handle in production (HFT continues)
                }
            }

            // Outbound Phase (Sending spike batches)
            if let Some(msg) = self.egress_pool.pop_ready() {
                // Decode batch to be fragmented and sent
                if let Ok((header, spikes)) = protocol::decode_spike_batch(&msg.buffer[..msg.size]) {
                    // Fragment batch into MTU-compliant chunks (e.g. 1400 bytes)
                    let mtu = 1400;
                    if let Ok(fragments) = protocol::fragment_spikes(header, spikes, mtu) {
                        for (chunk_header, chunk_spikes) in fragments {
                            // Serialize chunk header & spikes into send_buf
                            if let Ok(bytes_written) = protocol::encode_spike_batch(
                                &chunk_header,
                                chunk_spikes,
                                &mut send_buf,
                            ) {
                                // Lookup destination address in the routing table
                                if let Some(target_addr) = self.routes.get_address(chunk_header.dst_zone_hash) {
                                    let _ = self.socket.send_to(&send_buf[..bytes_written], target_addr);
                                }
                            }
                        }
                    }
                }

                // Return the message container back to the pool's free queue
                let _ = self.egress_pool.release(msg);
            }

            // Yield CPU control briefly if there's no data to process (Wait Strategy)
            std::thread::yield_now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wire::{SpikeBatchHeaderV2, SpikeEventV2};

    #[test]
    fn test_worker_epoch_validation_amnesia() {
        let routes = Arc::new(crate::routing::RoutingTable::new());
        let egress_pool = Arc::new(transport::EgressPool::new(10));
        let reassembly = protocol::ReassemblyBuffer::new(5);
        let current_epoch = Arc::new(AtomicU32::new(10));
        let shutdown = Arc::new(AtomicBool::new(false));

        // Create standard loopback UDP sockets
        let socket_receiver = transport::FastPathSocket::bind("127.0.0.1:0").unwrap();
        let receiver_addr = socket_receiver.socket.local_addr().unwrap();
        let socket_sender = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();

        let mut worker = UdpWorker::new(
            socket_receiver,
            routes,
            egress_pool,
            reassembly,
            current_epoch.clone(),
            shutdown.clone(),
        );

        // Send a packet from the past (epoch 2 while current is 10)
        let header = SpikeBatchHeaderV2 {
            src_zone_hash: 42,
            dst_zone_hash: 43,
            epoch: 2,
            chunk_idx: 0,
            total_chunks: 1,
        };
        let spikes = vec![SpikeEventV2 { ghost_id: 1, tick_offset: 10 }];
        let mut buf = vec![0u8; 1000];
        let size = protocol::encode_spike_batch(&header, &spikes, &mut buf).unwrap();
        socket_sender.send_to(&buf[..size], receiver_addr).unwrap();

        // Run one loop step manually or let worker process it
        // We set shutdown to true immediately after one check
        let shutdown_clone = shutdown.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            shutdown_clone.store(true, Ordering::Relaxed);
        });

        worker.run();

        // The slot for zone 42 should not have been updated since epoch 2 is AmnesiaDrop
        assert_eq!(worker.reassembly.slots[0].src_zone_hash, 0);
        assert_eq!(current_epoch.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_worker_epoch_self_healing() {
        let routes = Arc::new(crate::routing::RoutingTable::new());
        let egress_pool = Arc::new(transport::EgressPool::new(10));
        let reassembly = protocol::ReassemblyBuffer::new(5);
        let current_epoch = Arc::new(AtomicU32::new(10));
        let shutdown = Arc::new(AtomicBool::new(false));

        let socket_receiver = transport::FastPathSocket::bind("127.0.0.1:0").unwrap();
        let receiver_addr = socket_receiver.socket.local_addr().unwrap();
        let socket_sender = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();

        let mut worker = UdpWorker::new(
            socket_receiver,
            routes,
            egress_pool,
            reassembly,
            current_epoch.clone(),
            shutdown.clone(),
        );

        // Send a packet from the distant future (epoch 150 while current is 10)
        let header = SpikeBatchHeaderV2 {
            src_zone_hash: 42,
            dst_zone_hash: 43,
            epoch: 150,
            chunk_idx: 0,
            total_chunks: 1,
        };
        let spikes = vec![SpikeEventV2 { ghost_id: 1, tick_offset: 10 }];
        let mut buf = vec![0u8; 1000];
        let size = protocol::encode_spike_batch(&header, &spikes, &mut buf).unwrap();
        socket_sender.send_to(&buf[..size], receiver_addr).unwrap();

        let shutdown_clone = shutdown.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            shutdown_clone.store(true, Ordering::Relaxed);
        });

        worker.run();

        // The current_epoch should have fast-forwarded to 150
        assert_eq!(current_epoch.load(Ordering::Relaxed), 150);
    }
}
