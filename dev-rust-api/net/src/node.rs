use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicBool};
use std::net::SocketAddr;
use tokio::sync::broadcast::Sender;
use crate::worker::UdpWorker;

/// NetworkNode serves as the facade for L5 network operations.
pub struct NetworkNode;

impl NetworkNode {
    /// Start the network node servers and return the UDP fast path worker.
    ///
    /// This method starts/retrieves a tokio runtime context to spawn the slow-path
    /// servers (GeometryServer and TelemetryServer) in the background, keeping the
    /// data plane isolated for the returned UdpWorker.
    pub fn start(
        geometry_addr: SocketAddr,
        telemetry_tx: Sender<wire::TelemetryFrameHeader>,
        socket: transport::FastPathSocket,
        routes: Arc<crate::routing::RoutingTable>,
        egress_pool: Arc<transport::EgressPool>,
        reassembly: protocol::ReassemblyBuffer,
        current_epoch: Arc<AtomicU32>,
        shutdown: Arc<AtomicBool>,
    ) -> UdpWorker {
        // Retrieve or build a new tokio runtime
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let handle = rt.handle().clone();
                // Spawn thread to run the runtime indefinitely
                std::thread::spawn(move || {
                    rt.block_on(std::future::pending::<()>());
                });
                handle
            }
        };

        // Bind and spawn GeometryServer in the tokio runtime
        let geometry_listener = handle.block_on(async {
            tokio::net::TcpListener::bind(geometry_addr).await.unwrap()
        });
        let geometry_server = crate::geometry::GeometryServer::new(geometry_listener);
        handle.spawn(geometry_server.run());

        // Spawn TelemetryServer in the tokio runtime
        let telemetry_server = crate::telemetry::TelemetryServer::new(telemetry_tx);
        handle.spawn(async move {
            let _ = telemetry_server.start().await;
        });

        // Return the ready UdpWorker for isolated execution
        UdpWorker::new(
            socket,
            routes,
            egress_pool,
            reassembly,
            current_epoch,
            shutdown,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[test]
    fn test_node_start_facade() {
        let (tx, _rx) = broadcast::channel(16);
        let routes = Arc::new(crate::routing::RoutingTable::new());
        let egress_pool = Arc::new(transport::EgressPool::new(10));
        let reassembly = protocol::ReassemblyBuffer::new(5);
        let current_epoch = Arc::new(AtomicU32::new(1));
        let shutdown = Arc::new(AtomicBool::new(false));

        // Ephemeral ports
        let geometry_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let socket = transport::FastPathSocket::bind("127.0.0.1:0").unwrap();

        // Create a tokio runtime context for test
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();

        let _worker = NetworkNode::start(
            geometry_addr,
            tx,
            socket,
            routes,
            egress_pool,
            reassembly,
            current_epoch,
            shutdown,
        );
    }
}
