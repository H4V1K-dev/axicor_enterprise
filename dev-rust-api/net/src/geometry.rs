use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;

/// GeometryServer handles slow-path geometry synchronization (Night Phase).
///
/// # Slow/Fast Path Asymmetry (INV-NET-005)
/// Under INV-NET-005, the TCP listener is isolated to prevent head-of-line blocking
/// or latency spikes from slowing down the UDP hot path.
pub struct GeometryServer {
    pub listener: TcpListener,
}

impl GeometryServer {
    /// Create a new GeometryServer instance.
    pub fn new(listener: TcpListener) -> Self {
        Self { listener }
    }

    /// Run the server loop, accepting TCP connections and spawning handlers.
    pub async fn run(self) {
        loop {
            match self.listener.accept().await {
                Ok((mut stream, _addr)) => {
                    tokio::spawn(async move {
                        let mut buf = [0u8; 1024];
                        loop {
                            match stream.read(&mut buf).await {
                                Ok(0) => break, // Connection closed (EOF)
                                Ok(_) => {
                                    // Process geometry sync messages here (currently a stub)
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }
                Err(_) => {
                    // Prevent busy loop if accept fails repeatedly
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_geometry_server_connection() {
        // Bind to local ephemeral port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bound_addr = listener.local_addr().unwrap();

        let server = GeometryServer::new(listener);

        // Run server in background task
        let server_handle = tokio::spawn(server.run());

        // Connect to server
        let mut client = TcpStream::connect(bound_addr).await.unwrap();
        client.write_all(b"test data").await.unwrap();
        client.shutdown().await.unwrap();

        // Let the handler execute
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Clean up
        server_handle.abort();
    }
}
