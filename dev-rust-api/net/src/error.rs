use std::fmt;

/// Errors that can occur during network routing, coordinate barriers, and external IO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetError {
    /// Сосед не ответил на синхронизацию эпохи в рамках BSP_TIMEOUT_MS
    Timeout { zone_hash: u32 },
    /// Попытка отправить пакет в зону, отсутствующую в RCU-таблице
    RouteNotFound { zone_hash: u32 },
    /// Ошибка на уровне UDP/TCP сокетов и lock-free очередей
    Transport(transport::TransportError),
    /// Ошибка парсинга байтов, L7-фрагментации или валидации эпох
    Protocol(protocol::ProtocolError),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout { zone_hash } => write!(
                f,
                "Node synchronization timeout for zone hash: {}",
                zone_hash
            ),
            Self::RouteNotFound { zone_hash } => {
                write!(f, "Route not found for zone hash: {}", zone_hash)
            }
            Self::Transport(err) => write!(f, "Transport error: {}", err),
            Self::Protocol(err) => write!(f, "Protocol error: {}", err),
        }
    }
}

impl std::error::Error for NetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(err) => Some(err),
            Self::Protocol(err) => Some(err),
            _ => None,
        }
    }
}

impl From<transport::TransportError> for NetError {
    fn from(err: transport::TransportError) -> Self {
        Self::Transport(err)
    }
}

impl From<protocol::ProtocolError> for NetError {
    fn from(err: protocol::ProtocolError) -> Self {
        Self::Protocol(err)
    }
}
