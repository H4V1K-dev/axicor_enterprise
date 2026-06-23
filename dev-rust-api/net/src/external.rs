use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::net::{UdpSocket, SocketAddr};
use crossbeam::queue::SegQueue;
use wire::WireCast as _;

/// UDP External I/O Server for CartPole coordination.
pub struct ExternalIoServer {
    pub socket: UdpSocket,
    pub input_queue: Arc<SegQueue<[u8; 32]>>,
    pub global_dopamine: Arc<AtomicI32>,
    pub client_addr: std::sync::Mutex<Option<SocketAddr>>,
}

impl ExternalIoServer {
    /// Bind a new ExternalIoServer to the specified UDP address (e.g. "127.0.0.1:8081")
    pub fn bind(addr: &str) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        // Set read timeout so the loop can check shutdown flags or run without hard locks
        socket.set_read_timeout(Some(std::time::Duration::from_millis(500)))?;
        
        Ok(Self {
            socket,
            input_queue: Arc::new(SegQueue::new()),
            global_dopamine: Arc::new(AtomicI32::new(0)),
            client_addr: std::sync::Mutex::new(None),
        })
    }

    /// Run the UDP receiver loop (should be spawned on a background thread).
    pub fn run_rx_loop(&self, shutdown: Arc<std::sync::atomic::AtomicBool>) {
        let mut buf = [0u8; 65536];
        tracing::info!("[net-external] UDP IO Server listening on {}", self.socket.local_addr().unwrap());

        while !shutdown.load(Ordering::Relaxed) {
            match self.socket.recv_from(&mut buf) {
                Ok((len, src_addr)) => {
                    tracing::info!("[net-external] Received UDP packet: len={} from {}", len, src_addr);
                    if len < 20 {
                        continue;
                    }

                    // Decode and validate header
                    let header_raw: wire::ExternalIoHeader = match bytemuck::try_pod_read_unaligned(&buf[..20]) {
                        Ok(h) => h,
                        Err(e) => {
                            tracing::error!("[net-external] Header parse error: {:?}", e);
                            continue;
                        }
                    };
                    let header = header_raw.from_le();

                    // Check magic: "GSIO" is input
                    if header.magic != *b"GSIO" {
                        continue;
                    }

                    // Track client address for responses (override with port 8092 as per python client setup)
                    {
                        let mut addr_guard = self.client_addr.lock().unwrap();
                        let mut resp_addr = src_addr;
                        resp_addr.set_port(8092);
                        *addr_guard = Some(resp_addr);
                    }

                    // Update global dopamine level
                    self.global_dopamine.store(header.global_reward as i32, Ordering::Relaxed);

                    // Parse payload (inputs)
                    let payload_size = header.payload_size as usize;
                    let payload_data = &buf[20..20 + payload_size];
                    if payload_data.len() != payload_size {
                        continue;
                    }

                    // Push inputs into queue tick by tick (32 bytes per tick)
                    for chunk in payload_data.chunks_exact(32) {
                        let mut tick_data = [0u8; 32];
                        tick_data.copy_from_slice(chunk);
                        self.input_queue.push(tick_data);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                    // Timeout occurred, check shutdown flag and loop again
                }
                Err(e) => {
                    tracing::error!("[net-external] UDP receive error: {:?}", e);
                }
            }
        }
    }

    /// Block-wait and pop exactly `count` ticks (each 32 bytes) from the input queue.
    pub fn pop_inputs(&self, count: usize) -> Vec<u8> {
        let mut inputs = Vec::with_capacity(count * 32);
        for _ in 0..count {
            loop {
                if let Some(tick) = self.input_queue.pop() {
                    inputs.extend_from_slice(&tick);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_micros(100));
            }
        }
        inputs
    }

    /// Send motor outputs back to the client as a GSOO packet.
    pub fn send_outputs(&self, zone_hash: u32, output_payload: &[u8]) -> anyhow::Result<()> {
        let total_size = 20 + output_payload.len();
        let mut packet = vec![0u8; total_size];

        let header = wire::ExternalIoHeader {
            magic: *b"GSOO",
            zone_hash,
            matrix_hash: 0,
            payload_size: output_payload.len() as u32,
            global_reward: self.global_dopamine.load(Ordering::Relaxed) as i16,
            _padding: 0,
        };

        packet[..20].copy_from_slice(header.to_le().as_bytes());
        packet[20..].copy_from_slice(output_payload);

        let target_addr = {
            let addr_guard = self.client_addr.lock().unwrap();
            addr_guard.unwrap_or_else(|| "127.0.0.1:8092".parse().unwrap())
        };

        tracing::info!("[net-external] Sending GSOO response to {}: len={}", target_addr, packet.len());

        self.socket.send_to(&packet, target_addr)?;
        Ok(())
    }
}
