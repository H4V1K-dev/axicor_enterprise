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
