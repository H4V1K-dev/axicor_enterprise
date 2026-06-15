use std::net::SocketAddr;
use tokio::sync::broadcast::Sender;
use axum::{routing::get, Router};

/// TelemetryServer streams warp-aggregated metrics to clients.
///
/// # Lagging Receiver Pattern (E-126)
/// Under E-126, if telemetry clients fail to consume WebSocket data fast enough,
/// the broadcast channel overflows. We discard/ignore the send results to ensure
/// that slow telemetry consumers do not block or degrade the primary simulation/data plane thread.
pub struct TelemetryServer {
    pub broadcast_tx: Sender<wire::TelemetryFrameHeader>,
}

impl TelemetryServer {
    /// Create a new TelemetryServer instance.
    pub fn new(broadcast_tx: Sender<wire::TelemetryFrameHeader>) -> Self {
        Self { broadcast_tx }
    }

    /// Broadcast a telemetry frame, ignoring lagging receiver errors (E-126).
    pub fn broadcast(&self, frame: wire::TelemetryFrameHeader) {
        let _ = self.broadcast_tx.send(frame);
    }

    /// Start the telemetry web server on an ephemeral port.
    ///
    /// Spawns the axum runner task in the background and returns the bound socket address.
    pub async fn start(self) -> SocketAddr {
        let app = Router::new().route("/", get(|| async { "Telemetry Active" }));

        // Bind to localhost ephemeral port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bound_addr = listener.local_addr().unwrap();

        // Spawn axum task in the background
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        bound_addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;
    use wire::TelemetryFrameHeader;

    #[tokio::test]
    async fn test_telemetry_server_ephemeral() {
        let (tx, mut rx1) = broadcast::channel(2);
        let tx_clone = tx.clone();
        let server = TelemetryServer::new(tx);

        // Start server and get bound address
        let bound_addr = server.start().await;
        assert_ne!(bound_addr.port(), 0);

        // Force a lagging receiver scenario (fill queue of 2 elements, send 3)
        // Verify that broadcast does not block and handles lagging receiver (E-126) safely
        let frame = TelemetryFrameHeader {
            magic: *b"TELE",
            tick: 100,
            count: 0,
            _padding: 0,
        };

        let broadcaster = TelemetryServer::new(tx_clone);
        broadcaster.broadcast(frame);
        broadcaster.broadcast(frame);
        // This third send should drop the first elements for rx1 without blocking
        broadcaster.broadcast(frame);

        // Attempting to read should result in a Lagged error due to the overflow
        let recv_res = rx1.recv().await;
        assert!(recv_res.is_err());
        assert!(matches!(recv_res.unwrap_err(), broadcast::error::RecvError::Lagged(_)));
    }
}
