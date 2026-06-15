use std::fmt;

/// Errors that can occur during low-level transport operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// Запрошенная емкость буфера не является степенью двойки
    InvalidCapacity,
    /// Очередь кольцевого буфера полностью переполнена (сеть лагает)
    QueueFull,
    /// Системный сокет не готов к неблокирующей транзакции (EWOULDBLOCK)
    SocketWouldBlock,
    /// Размер полученного пакета превышает системный лимит пре-аллоцированного буфера
    BufferOverflow,
    /// Фатальная ошибка I/O операционной системы (например, EBADF)
    IoError(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity => write!(f, "Requested buffer capacity is not a power of two"),
            Self::QueueFull => write!(f, "Ring buffer queue is completely full"),
            Self::SocketWouldBlock => write!(f, "Socket operation would block"),
            Self::BufferOverflow => write!(f, "Buffer overflow: packet size exceeds limits"),
            Self::IoError(err) => write!(f, "OS I/O error: {}", err),
        }
    }
}

impl std::error::Error for TransportError {}
