use crate::error::TransportError;

/// Maximum size of the raw network buffer for UDP datagrams.
pub const UDP_BUFFER_SIZE: usize = 65536;

/// DTO-container for a non-blocking outgoing network packet.
pub struct EgressMessage {
    /// Pre-allocated buffer containing strictly zero-initialized memory of size UDP_BUFFER_SIZE.
    pub buffer: Vec<u8>,
    /// Actual payload size of the packet.
    pub size: usize,
    /// Destination socket address.
    pub target: std::net::SocketAddr,
}

impl EgressMessage {
    /// Create a new egress message container.
    pub fn new() -> Self {
        Self {
            buffer: vec![0u8; UDP_BUFFER_SIZE],
            size: 0,
            target: std::net::SocketAddr::from(([0, 0, 0, 0], 0)),
        }
    }
}

impl Default for EgressMessage {
    fn default() -> Self {
        Self::new()
    }
}

/// EgressPool acts as a lock-free pool of pre-allocated egress message buffers.
///
/// # Egress pool zero-allocation constraint (INV-TRANS-002)
/// Under INV-TRANS-002, the EgressPool manages lock-free queues (ArrayQueue) containing
/// pre-allocated buffers. The orchestrator/ HFT-loop thread acquires free message containers
/// from `free_queue`, serializes the spike events directly into the pre-allocated slice without
/// heap allocation (malloc), and throws the populated container into `ready_queue`. A dedicated background
/// thread pops packets from `ready_queue` and executes system `send_to` calls. This split prevents
/// OS-level system calls and heap allocations from introducing jitter into the high-frequency GPU loop.
pub struct EgressPool {
    /// Lock-free queue of free/available message buffers.
    pub free_queue: crossbeam::queue::ArrayQueue<EgressMessage>,
    /// Lock-free queue of ready-to-send message buffers.
    pub ready_queue: crossbeam::queue::ArrayQueue<EgressMessage>,
}

impl EgressPool {
    /// Create a new EgressPool initializing all message containers.
    pub fn new(capacity: usize) -> Self {
        let free_queue = crossbeam::queue::ArrayQueue::new(capacity);
        let ready_queue = crossbeam::queue::ArrayQueue::new(capacity);

        for _ in 0..capacity {
            let _ = free_queue.push(EgressMessage::new());
        }

        Self {
            free_queue,
            ready_queue,
        }
    }

    /// Try to acquire a free message container from the pool.
    pub fn acquire(&self) -> Option<EgressMessage> {
        self.free_queue.pop()
    }

    /// Return a message container back to the free pool.
    ///
    /// # Errors
    /// Returns `TransportError::QueueFull` if the free queue is full.
    pub fn release(&self, mut msg: EgressMessage) -> Result<(), TransportError> {
        msg.size = 0;
        self.free_queue.push(msg).map_err(|_| TransportError::QueueFull)
    }

    /// Push a prepared message container to the ready queue.
    ///
    /// # Errors
    /// Returns `TransportError::QueueFull` if the ready queue is full.
    pub fn push_ready(&self, msg: EgressMessage) -> Result<(), TransportError> {
        self.ready_queue.push(msg).map_err(|_| TransportError::QueueFull)
    }

    /// Pop a message container from the ready queue.
    pub fn pop_ready(&self) -> Option<EgressMessage> {
        self.ready_queue.pop()
    }
}
