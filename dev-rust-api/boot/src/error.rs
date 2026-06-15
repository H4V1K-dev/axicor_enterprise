use std::fmt;

/// Custom error type representing all possible failures during the node boot pipeline.
#[derive(Debug)]
pub enum BootError {
    /// Ошибка монтирования архива VFS
    VfsMount(vfs::VfsError),
    /// Ошибка манифеста (отсутствует или поврежден)
    Manifest(String),
    /// Ошибка при извлечении файлов в tmpfs
    RomExtract(std::io::Error),
    /// Нарушение выравнивания байт
    CabiGuard { expected: usize, actual: usize },
    /// Недостаточно VRAM для аллокации
    VramExhausted,
    /// Ошибка при инициализации барьера или сети
    NetworkInit(String),
}

impl fmt::Display for BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VfsMount(err) => write!(f, "VFS mount failure: {}", err),
            Self::Manifest(err) => write!(f, "Manifest error: {}", err),
            Self::RomExtract(err) => write!(f, "ROM extraction failure: {}", err),
            Self::CabiGuard { expected, actual } => {
                write!(
                    f,
                    "C-ABI alignment violation: expected alignment of {} bytes, actual was {}",
                    expected, actual
                )
            }
            Self::VramExhausted => write!(f, "VRAM allocation failed: out of memory"),
            Self::NetworkInit(err) => write!(f, "Network initialization failure: {}", err),
        }
    }
}

impl std::error::Error for BootError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::VfsMount(err) => Some(err),
            Self::RomExtract(err) => Some(err),
            _ => None,
        }
    }
}

impl From<vfs::VfsError> for BootError {
    fn from(err: vfs::VfsError) -> Self {
        Self::VfsMount(err)
    }
}

impl From<std::io::Error> for BootError {
    fn from(err: std::io::Error) -> Self {
        Self::RomExtract(err)
    }
}
