//! Socket-based inter-process communication for the Night Phase.

use crate::error::IpcError;
use bytemuck::Zeroable;

#[cfg(target_os = "linux")]
use std::os::unix::net::{UnixListener, UnixStream};

#[cfg(target_os = "windows")]
use std::net::{TcpListener, TcpStream};

/// Server listener for baking coordination.
pub struct BakerServer {
    #[cfg(target_os = "linux")]
    listener: UnixListener,
    #[cfg(target_os = "windows")]
    listener: TcpListener,
}

/// Client connection for triggering baking.
pub struct BakerClient {
    #[cfg(target_os = "linux")]
    stream: UnixStream,
    #[cfg(target_os = "windows")]
    stream: TcpStream,
}

/// Active connection accepted by the server.
pub struct BakerServerConnection {
    #[cfg(target_os = "linux")]
    stream: UnixStream,
    #[cfg(target_os = "windows")]
    stream: TcpStream,
}

impl BakerServer {
    /// Bind a new BakerServer listener for the specified zone hash.
    ///
    /// # Invariants and Edge Cases
    /// - **INV-IPC-004**: UDS Access Isolation - Unix Domain Sockets must be created strictly with 0o700 permissions to prevent unauthorized access.
    /// - **E-033**: If the socket file exists from a previous run, a clean unlink must be performed on Unix before binding. If a TCP port is in use on Windows, `IpcError::AddrInUse` is returned.
    #[cfg(target_os = "linux")]
    pub fn bind(zone_hash: u32) -> Result<Self, IpcError> {
        let path = crate::utils::default_socket_path(zone_hash);
        
        // E-033: Unlink socket file if it exists
        if std::path::Path::new(&path).exists() {
            std::fs::remove_file(&path).map_err(IpcError::Io)?;
        }
        
        let listener = UnixListener::bind(&path).map_err(IpcError::Io)?;
        
        // INV-IPC-004: Set permissions strictly to 0o700
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).map_err(IpcError::Io)?;
        
        Ok(Self { listener })
    }

    /// Bind a new BakerServer listener for the specified zone hash.
    ///
    /// # Invariants and Edge Cases
    /// - **INV-IPC-004**: UDS Access Isolation - Unix Domain Sockets must be created strictly with 0o700 permissions to prevent unauthorized access.
    /// - **E-033**: If the socket file exists from a previous run, a clean unlink must be performed on Unix before binding. If a TCP port is in use on Windows, `IpcError::AddrInUse` is returned.
    #[cfg(target_os = "windows")]
    pub fn bind(zone_hash: u32) -> Result<Self, IpcError> {
        let addr = crate::utils::default_socket_path(zone_hash);
        let listener = TcpListener::bind(&addr).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                IpcError::AddrInUse
            } else {
                IpcError::Io(e)
            }
        })?;
        Ok(Self { listener })
    }

    /// Accept an incoming connection.
    #[cfg(target_os = "linux")]
    pub fn accept(&self) -> Result<BakerServerConnection, IpcError> {
        let (stream, _) = self.listener.accept().map_err(IpcError::Io)?;
        Ok(BakerServerConnection { stream })
    }

    /// Accept an incoming connection.
    #[cfg(target_os = "windows")]
    pub fn accept(&self) -> Result<BakerServerConnection, IpcError> {
        let (stream, _) = self.listener.accept().map_err(IpcError::Io)?;
        stream.set_nodelay(true).map_err(IpcError::Io)?;
        Ok(BakerServerConnection { stream })
    }
}

impl BakerClient {
    /// Connect to a BakerServer for the specified zone hash.
    #[cfg(target_os = "linux")]
    pub fn connect(zone_hash: u32) -> Result<Self, IpcError> {
        let path = crate::utils::default_socket_path(zone_hash);
        let stream = UnixStream::connect(&path).map_err(IpcError::Io)?;
        Ok(Self { stream })
    }

    /// Connect to a BakerServer for the specified zone hash.
    #[cfg(target_os = "windows")]
    pub fn connect(zone_hash: u32) -> Result<Self, IpcError> {
        let addr = crate::utils::default_socket_path(zone_hash);
        let stream = TcpStream::connect(&addr).map_err(IpcError::Io)?;
        stream.set_nodelay(true).map_err(IpcError::Io)?;
        Ok(Self { stream })
    }

    /// Trigger the Night Phase computation, sending a BakeRequest and handovers,
    /// and blocks until response with AxonHandoverAcks is received.
    pub fn trigger_night_phase(
        &mut self,
        req: &wire::BakeRequest,
        handovers: &[wire::AxonHandoverEvent],
    ) -> Result<Vec<wire::AxonHandoverAck>, IpcError> {
        use std::io::{Read, Write};

        // Write BakeRequest
        self.stream.write_all(bytemuck::bytes_of(req)).map_err(IpcError::Io)?;

        // Write handovers count
        let count = handovers.len() as u32;
        self.stream.write_all(bytemuck::bytes_of(&count)).map_err(IpcError::Io)?;

        // Write AxonHandoverEvent slice
        self.stream.write_all(bytemuck::cast_slice(handovers)).map_err(IpcError::Io)?;

        // Read response: BAKE_READY_MAGIC (BKOK)
        let mut magic_buf = [0u8; 4];
        self.stream.read_exact(&mut magic_buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                IpcError::ConnectionReset
            } else {
                IpcError::Io(e)
            }
        })?;
        let magic = u32::from_le_bytes(magic_buf);
        if magic != 0x424B4F4B {
            return Err(IpcError::InvalidProtocolPacket);
        }

        // Read response count
        let mut ack_count_buf = [0u8; 4];
        self.stream.read_exact(&mut ack_count_buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                IpcError::ConnectionReset
            } else {
                IpcError::Io(e)
            }
        })?;
        let ack_count = u32::from_le_bytes(ack_count_buf) as usize;

        // Read AxonHandoverAck slice
        let mut acks = vec![wire::AxonHandoverAck::zeroed(); ack_count];
        if ack_count > 0 {
            let bytes = bytemuck::cast_slice_mut::<wire::AxonHandoverAck, u8>(&mut acks);
            self.stream.read_exact(bytes).map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    IpcError::ConnectionReset
                } else {
                    IpcError::Io(e)
                }
            })?;
        }

        Ok(acks)
    }
}

impl BakerServerConnection {
    /// Receive a BakeRequest and associated AxonHandoverEvents.
    pub fn recv_request(&mut self) -> Result<(wire::BakeRequest, Vec<wire::AxonHandoverEvent>), IpcError> {
        use std::io::Read;

        // Read BakeRequest
        let mut req_buf = [0u8; 16];
        self.stream.read_exact(&mut req_buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                IpcError::ConnectionReset
            } else {
                IpcError::Io(e)
            }
        })?;
        let req = *bytemuck::try_from_bytes::<wire::BakeRequest>(&req_buf)
            .map_err(|_| IpcError::InvalidProtocolPacket)?;

        // Read count
        let mut count_buf = [0u8; 4];
        self.stream.read_exact(&mut count_buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                IpcError::ConnectionReset
            } else {
                IpcError::Io(e)
            }
        })?;
        let count = u32::from_le_bytes(count_buf) as usize;

        // Read handovers
        let mut handovers = vec![wire::AxonHandoverEvent::zeroed(); count];
        if count > 0 {
            let bytes = bytemuck::cast_slice_mut::<wire::AxonHandoverEvent, u8>(&mut handovers);
            self.stream.read_exact(bytes).map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    IpcError::ConnectionReset
                } else {
                    IpcError::Io(e)
                }
            })?;
        }

        Ok((req, handovers))
    }

    /// Send AxonHandoverAcks back to the client.
    pub fn send_response(&mut self, acks: &[wire::AxonHandoverAck]) -> Result<(), IpcError> {
        use std::io::Write;

        // Write BAKE_READY_MAGIC
        let magic: u32 = 0x424B4F4B;
        self.stream.write_all(bytemuck::bytes_of(&magic)).map_err(IpcError::Io)?;

        // Write count
        let count = acks.len() as u32;
        self.stream.write_all(bytemuck::bytes_of(&count)).map_err(IpcError::Io)?;

        // Write acks
        self.stream.write_all(bytemuck::cast_slice(acks)).map_err(IpcError::Io)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_socket_loopback_exchange() {
        let zone_hash = 999;
        
        // Bind server
        let server = BakerServer::bind(zone_hash).expect("Failed to bind server");

        // Spawn background server thread to handle request
        let server_thread = thread::spawn(move || {
            let mut conn = server.accept().expect("Failed to accept connection");
            let (req, handovers) = conn.recv_request().expect("Failed to receive request");
            
            // Validate request
            assert_eq!(req.magic, *b"BAKE");
            assert_eq!(req.zone_hash, zone_hash);
            assert_eq!(req.current_tick, 100);
            assert_eq!(req.prune_threshold, -10);
            assert_eq!(req.max_sprouts, 5);
            
            // Validate handovers
            assert_eq!(handovers.len(), 1);
            assert_eq!(handovers[0].origin_zone_hash, zone_hash);
            assert_eq!(handovers[0].local_axon_id, 12345);

            // Respond with some acks
            let ack = wire::AxonHandoverAck {
                target_zone_hash: zone_hash,
                receiver_zone_hash: zone_hash + 1,
                src_axon_id: 12345,
                dst_ghost_id: 67890,
            };
            conn.send_response(&[ack]).expect("Failed to send response");
        });

        // Connect client
        let mut client = BakerClient::connect(zone_hash).expect("Failed to connect client");
        
        let req = wire::BakeRequest {
            magic: *b"BAKE",
            zone_hash,
            current_tick: 100,
            prune_threshold: -10,
            max_sprouts: 5,
        };
        
        let handover = wire::AxonHandoverEvent {
            origin_zone_hash: zone_hash,
            local_axon_id: 12345,
            entry_x: 1,
            entry_y: 2,
            vector_x: 0,
            vector_y: 0,
            vector_z: 0,
            type_mask: 0,
            remaining_length: 10,
            entry_z: 3,
            _padding: 0,
        };

        // Trigger night phase
        let acks = client.trigger_night_phase(&req, &[handover]).expect("Failed to trigger night phase");

        // Join server thread
        server_thread.join().expect("Server thread panicked");

        // Validate acks
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].target_zone_hash, zone_hash);
        assert_eq!(acks[0].receiver_zone_hash, zone_hash + 1);
        assert_eq!(acks[0].src_axon_id, 12345);
        assert_eq!(acks[0].dst_ghost_id, 67890);
    }

    #[test]
    fn test_addr_in_use() {
        let zone_hash = 54321;
        // First bind
        let _server1 = BakerServer::bind(zone_hash).expect("Failed to bind first server");
        // Second bind on same zone_hash should fail with AddrInUse
        let server2_res = BakerServer::bind(zone_hash);
        
        #[cfg(target_os = "windows")]
        {
            assert!(matches!(server2_res, Err(IpcError::AddrInUse)));
        }
        #[cfg(target_os = "linux")]
        {
            // On Linux, the second bind will try to unlink the file first.
            // If the socket is bound and listening, on Linux UnixDomainSocket, unlinking is allowed, but the old socket listener remains bound, and a new bind on the same path will succeed (stealing the path).
            // So we don't assert AddrInUse on Linux for Unix Domain Sockets.
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_uds_permissions_isolation() {
        use std::os::unix::fs::PermissionsExt;
        
        let zone_hash = 12345;
        let path = crate::utils::default_socket_path(zone_hash);
        
        // Bind server
        let _server = BakerServer::bind(zone_hash).expect("Failed to bind server");
        
        // Verify socket file permissions
        let metadata = std::fs::metadata(&path).expect("Failed to get socket metadata");
        let permissions = metadata.permissions();
        let mode = permissions.mode() & 0o777;
        assert_eq!(mode, 0o700);
        
        // Clean up
        let _ = std::fs::remove_file(&path);
    }
}
