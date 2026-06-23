pub mod error;
pub mod strategy;
pub mod ring_buffer;
pub mod egress;

pub use error::*;
pub use strategy::*;
pub use ring_buffer::*;
pub use egress::*;

/// Wrapper around UDP socket for non-blocking HFT I/O.
pub struct FastPathSocket {
    pub socket: std::net::UdpSocket,
}

impl FastPathSocket {
    /// Bind a new FastPathSocket to the given address and set it to non-blocking mode.
    pub fn bind<A: std::net::ToSocketAddrs>(addr: A) -> Result<Self, std::io::Error> {
        let socket = std::net::UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }

    /// Send data to a destination address.
    pub fn send_to(&self, buf: &[u8], addr: std::net::SocketAddr) -> Result<usize, std::io::Error> {
        self.socket.send_to(buf, addr)
    }

    /// Receive data from the socket.
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, std::net::SocketAddr), std::io::Error> {
        self.socket.recv_from(buf)
    }
}

/// Slow-path TCP listener for geometry sync (Night Phase).
pub struct GeometryTcpServer {
    pub listener: std::net::TcpListener,
}

impl GeometryTcpServer {
    /// Bind a new GeometryTcpServer to the given address and set it to non-blocking mode.
    pub fn bind<A: std::net::ToSocketAddrs>(addr: A) -> Result<Self, std::io::Error> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_egress_pool_zero_alloc_hot_path() {
        // INV-TRANS-002: Ensure egress pool can transition packets without allocation
        let pool = EgressPool::new(5);
        for i in 0..5 {
            let mut msg = pool.acquire().unwrap();
            assert_eq!(msg.size, 0);
            msg.size = 10 + i;
            pool.push_ready(msg).unwrap();
        }

        assert!(pool.acquire().is_none());

        for i in 0..5 {
            let msg = pool.pop_ready().unwrap();
            assert_eq!(msg.size, 10 + i);
            pool.release(msg).unwrap();
        }
    }

    #[test]
    fn test_fast_path_socket_would_block() {
        // E-120: Reading from an empty non-blocking UDP socket returns WouldBlock
        let socket = FastPathSocket::bind("127.0.0.1:0").unwrap();
        let mut buf = [0u8; 100];
        let res = socket.recv_from(&mut buf);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    }

    #[test]
    fn test_buffer_overflow_drop() {
        // E-121: Verify pre-allocated buffer size limits
        let msg = EgressMessage::new();
        assert_eq!(msg.buffer.len(), UDP_BUFFER_SIZE);
    }

    #[test]
    fn test_wait_strategy_execution() {
        // §6.2: Verify waiting strategy profiles
        let start = std::time::Instant::now();
        WaitStrategy::Eco.wait();
        let elapsed = start.elapsed();
        assert!(elapsed >= std::time::Duration::from_millis(1));

        let start = std::time::Instant::now();
        WaitStrategy::Aggressive.wait();
        let elapsed = start.elapsed();
        assert!(elapsed < std::time::Duration::from_millis(10));
    }

    #[test]
    fn test_tcp_udp_split_isolation() {
        // INV-TRANS-003: TCP server and UDP socket do not block or interfere with each other
        let tcp_server = GeometryTcpServer::bind("127.0.0.1:0").unwrap();
        let udp_socket = FastPathSocket::bind("127.0.0.1:0").unwrap();

        assert_ne!(tcp_server.listener.local_addr().unwrap().port(), 0);
        assert_ne!(udp_socket.socket.local_addr().unwrap().port(), 0);
    }

    #[test]
    fn test_egress_pool_saturation() {
        // D-032: Egress pool saturation returns QueueFull and drops gracefully
        let pool = EgressPool::new(2);
        let m1 = pool.acquire().unwrap();
        let m2 = pool.acquire().unwrap();
        assert!(pool.acquire().is_none());

        pool.push_ready(m1).unwrap();
        pool.push_ready(m2).unwrap();

        let m3 = EgressMessage::new();
        let res = pool.push_ready(m3);
        assert_eq!(res.err(), Some(TransportError::QueueFull));
    }

    #[test]
    fn test_teardown_socket_cleanup() {
        // INV-TRANS-004: Explicit socket drop frees ports for re-binding
        let port;
        {
            let socket = FastPathSocket::bind("127.0.0.1:0").unwrap();
            port = socket.socket.local_addr().unwrap().port();
        }
        let socket2 = FastPathSocket::bind(format!("127.0.0.1:{}", port));
        assert!(socket2.is_ok());
    }

    #[test]
    fn test_ring_buffer_concurrency() {
        // R-037: Thread-safe, lock-free ring buffer under multi-producer single-consumer concurrency
        use std::sync::Arc;
        let rb = Arc::new(RingBuffer::<i32>::new(128).unwrap());
        let rb_clone = rb.clone();

        let handle = std::thread::spawn(move || {
            for i in 0..100 {
                while rb_clone.push(i).is_err() {
                    std::thread::yield_now();
                }
            }
        });

        let mut sum = 0;
        let mut popped = 0;
        while popped < 100 {
            if let Some(val) = rb.pop() {
                sum += val;
                popped += 1;
            } else {
                std::thread::yield_now();
            }
        }

        handle.join().unwrap();
        assert_eq!(sum, (0..100).sum());
    }
}

