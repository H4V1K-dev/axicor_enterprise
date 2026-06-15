use core::fmt;

/// Errors that can occur during protocol-level serialization, deserialization, L7 fragmentation, and reassembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Неверный бинарный идентификатор (magic number)
    InvalidMagic { expected: [u8; 4], actual: [u8; 4] },
    /// Сетевой буфер меньше ожидаемого размера C-ABI структуры (оборванный пакет)
    BufferTooSmall { expected: usize, actual: usize },
    /// Стартовый адрес слайса памяти не кратен аппаратному align_of (риск Unaligned Access)
    AlignmentMismatch,
    /// Недопустимый размер MTU (вызывает деление на 0 в расчетах фрагментации)
    InvalidMtu { mtu: usize, min_required: usize },
    /// Заявленное число чанков превышает аппаратный лимит битовой маски в 1024
    InvalidChunkCount { total_chunks: u16, max_allowed: u16 },
    /// Индекс фрагмента в пакете больше либо равен total_chunks
    InvalidFragmentIndex { index: u16, total: u16 },
    /// Фрагмент с таким индексом уже зафиксирован в битовой маске (O(1) коллизия)
    DuplicateFragment { batch_id: u32, chunk_idx: u16 },
    /// Суммарный объем спайков в фрагменте превышает максимальную емкость батча
    BatchCapacityExceeded { max_spikes: usize, actual_spikes: usize },
    /// Эпоха пакета безнадежно устарела (Biological Amnesia)
    EpochTooOld { packet_epoch: u32, current_epoch: u32 },
    /// Несовпадение кластерного секрета в управляющем пакете
    AuthFailure,
    /// Кольцевой буфер переполнен "висячими" сессиями сборок (сетевой мусор)
    ReassemblyBufferFull,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic { expected, actual } => write!(
                f,
                "Invalid magic number: expected {:?}, actual {:?}",
                expected, actual
            ),
            Self::BufferTooSmall { expected, actual } => write!(
                f,
                "Buffer too small: expected at least {} bytes, actual {}",
                expected, actual
            ),
            Self::AlignmentMismatch => write!(f, "Pointer alignment mismatch (risk of unaligned access)"),
            Self::InvalidMtu { mtu, min_required } => write!(
                f,
                "Invalid MTU: {}, must be at least {}",
                mtu, min_required
            ),
            Self::InvalidChunkCount { total_chunks, max_allowed } => write!(
                f,
                "Invalid chunk count: {} exceeds maximum allowed {}",
                total_chunks, max_allowed
            ),
            Self::InvalidFragmentIndex { index, total } => write!(
                f,
                "Invalid fragment index: {} is out of bounds for total {}",
                index, total
            ),
            Self::DuplicateFragment { batch_id, chunk_idx } => write!(
                f,
                "Duplicate fragment detected: batch_id {}, chunk_idx {}",
                batch_id, chunk_idx
            ),
            Self::BatchCapacityExceeded { max_spikes, actual_spikes } => write!(
                f,
                "Batch capacity exceeded: max spikes {}, actual spikes {}",
                max_spikes, actual_spikes
            ),
            Self::EpochTooOld { packet_epoch, current_epoch } => write!(
                f,
                "Epoch too old: packet epoch {}, current epoch {}",
                packet_epoch, current_epoch
            ),
            Self::AuthFailure => write!(f, "Authentication failure: cluster secret mismatch"),
            Self::ReassemblyBufferFull => write!(f, "Reassembly buffer is full of incomplete sessions"),
        }
    }
}

impl core::error::Error for ProtocolError {}
