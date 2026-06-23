pub mod error;
pub mod routing;
pub mod bsp;
pub mod io;
pub mod worker;
pub mod geometry;
pub mod telemetry;
pub mod node;
pub mod external;

pub use error::*;
pub use routing::*;
pub use bsp::*;
pub use io::*;
pub use worker::*;
pub use geometry::*;
pub use telemetry::*;
pub use node::*;
pub use external::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bsp_barrier_timeout() {
        // INV-NET-004, E-122: If expected peers are not reached within BSP_TIMEOUT_MS, returns Timeout
        let barrier = BspBarrier::new(0, 1, transport::WaitStrategy::Eco);
        let start = std::time::Instant::now();
        let res = barrier.sync_and_swap(0);
        assert!(res.is_err());
        assert!(start.elapsed() >= std::time::Duration::from_millis(500));
        assert!(barrier.is_poisoned.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_control_data_plane_isolation() {
        // INV-NET-005, D-033: TCP server and UDP socket are isolated and run concurrently without deadlock
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = GeometryServer::new(listener);
        let handle = tokio::spawn(server.run());

        let sock = transport::FastPathSocket::bind("127.0.0.1:0").unwrap();
        let mut buf = [0u8; 100];
        let res = sock.recv_from(&mut buf);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        handle.abort();
    }

    #[test]
    fn test_distributed_deadlock_prevention() {
        // INV-NET-004: Deadlock prevention works when expecting multiple peers but some fail
        let barrier = BspBarrier::new(0, 2, transport::WaitStrategy::Eco);
        barrier.increment_completed_peers();
        let start = std::time::Instant::now();
        let res = barrier.sync_and_swap(0);
        assert!(res.is_err());
        assert!(start.elapsed() >= std::time::Duration::from_millis(500));
    }

    #[test]
    fn test_sensor_io_swapchain() {
        // R-040: Verify transpose operations on IoServer
        let mut io_server = IoServer::new();
        let soa_history = vec![0u8; 100];
        let res = io_server.send_output_batch(&soa_history, 10, 10);
        assert!(res.is_ok());
        assert_eq!(io_server.transpose_buffer.len(), 100);
    }

    #[test]
    fn test_reassembly_buffer_eviction() {
        // D-034: Reassembly buffer eviction clears out slots safely
        let mut buffer = protocol::ReassemblyBuffer::new(1);
        buffer.slots[0].reset(100, 1, 5);
        assert_eq!(buffer.slots[0].src_zone_hash, 100);
        buffer.evict_slot(0);
        assert_eq!(buffer.slots[0].src_zone_hash, 0);
    }
}

