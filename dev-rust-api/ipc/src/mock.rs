//! Mock shared memory allocator for testing the topology daemon in isolation.

use crate::error::IpcError;
use crate::shm::MappedShm;

/// Generator of mock SHM segments to simulate Night Phase triggers.
pub struct MockShmAllocator;

impl MockShmAllocator {
    /// Cold starts a mock SHM segment and triggers the Night Phase by setting state to NightStart.
    #[cfg(target_os = "linux")]
    pub fn allocate_and_trigger(zone_hash: u32, padded_n: usize) -> Result<MappedShm, IpcError> {
        use std::os::unix::io::FromRawFd;

        let shm_size = layout::shm_size(padded_n);
        let offsets = layout::compute_state_offsets(padded_n);
        let required_size = 128 + offsets.total_size + 10000 * 20 + 10000 * 12;
        let final_shm_size = shm_size.max(required_size);

        let path = crate::utils::shm_file_path(zone_hash);
        let path_str = path.to_str().ok_or_else(|| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))
        })?;
        let c_path = std::ffi::CString::new(path_str).map_err(|_| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Null byte in path"))
        })?;

        let fd = crate::platform::create_clean_shm(&c_path, final_shm_size)?;
        let file = unsafe { std::fs::File::from_raw_fd(fd) };

        let mapped = MappedShm::new(&file, final_shm_size)?;
        
        // Initialize header
        let hdr = unsafe { &mut *(mapped.mmap.as_ptr() as *mut layout::ShmHeader) };
        hdr.magic = layout::SHM_MAGIC;
        hdr.version = layout::SHM_VERSION;
        hdr.padded_n = padded_n as u32;
        hdr.dendrite_slots = layout::MAX_DENDRITES as u32;

        hdr.weights_offset = (offsets.dendrite_weights + 128) as u32;
        hdr.targets_offset = (offsets.dendrite_targets + 128) as u32;
        hdr.flags_offset = (offsets.flags + 128) as u32;
        hdr.voltage_offset = (offsets.soma_voltage + 128) as u32;
        hdr.threshold_offset_offset = (offsets.threshold_offset + 128) as u32;
        hdr.timers_offset = (offsets.timers + 128) as u32;

        let handovers_offset = 128 + offsets.total_size;
        hdr.handovers_offset = handovers_offset as u32;
        hdr.handovers_count = 10000;

        let prunes_offset = handovers_offset + 10000 * 20;
        hdr.prunes_offset = prunes_offset as u32;
        hdr.prunes_count = 10000;

        // Force transition to NightStart to trigger the daemon
        hdr.state = layout::ShmState::NightStart as u8;

        Ok(mapped)
    }

    /// Cold starts a mock SHM segment and triggers the Night Phase by setting state to NightStart.
    #[cfg(not(target_os = "linux"))]
    pub fn allocate_and_trigger(zone_hash: u32, padded_n: usize) -> Result<MappedShm, IpcError> {
        let shm_size = layout::shm_size(padded_n);
        let offsets = layout::compute_state_offsets(padded_n);
        let required_size = 128 + offsets.total_size + 10000 * 20 + 10000 * 12;
        let final_shm_size = shm_size.max(required_size);

        let path = crate::utils::shm_file_path(zone_hash);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.set_len(final_shm_size as u64)?;

        let mapped = MappedShm::new(&file, final_shm_size)?;

        // Initialize header
        let hdr = unsafe { &mut *(mapped.mmap.as_ptr() as *mut layout::ShmHeader) };
        hdr.magic = layout::SHM_MAGIC;
        hdr.version = layout::SHM_VERSION;
        hdr.padded_n = padded_n as u32;
        hdr.dendrite_slots = layout::MAX_DENDRITES as u32;

        hdr.weights_offset = (offsets.dendrite_weights + 128) as u32;
        hdr.targets_offset = (offsets.dendrite_targets + 128) as u32;
        hdr.flags_offset = (offsets.flags + 128) as u32;
        hdr.voltage_offset = (offsets.soma_voltage + 128) as u32;
        hdr.threshold_offset_offset = (offsets.threshold_offset + 128) as u32;
        hdr.timers_offset = (offsets.timers + 128) as u32;

        let handovers_offset = 128 + offsets.total_size;
        hdr.handovers_offset = handovers_offset as u32;
        hdr.handovers_count = 10000;

        let prunes_offset = handovers_offset + 10000 * 20;
        hdr.prunes_offset = prunes_offset as u32;
        hdr.prunes_count = 10000;

        // Force transition to NightStart to trigger the daemon
        hdr.state = layout::ShmState::NightStart as u8;

        Ok(mapped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_allocation_triggers_night_phase() {
        let zone_hash = 889;
        let padded_n = 1024;
        
        let mapped = MockShmAllocator::allocate_and_trigger(zone_hash, padded_n)
            .expect("Failed to allocate and trigger mock SHM");
            
        let hdr = unsafe { &*(mapped.mmap.as_ptr() as *const layout::ShmHeader) };
        assert_eq!(hdr.magic, layout::SHM_MAGIC);
        assert_eq!(hdr.version, layout::SHM_VERSION);
        assert_eq!(hdr.state, layout::ShmState::NightStart as u8);
        assert_eq!(hdr.padded_n, padded_n as u32);
        
        let offsets = layout::compute_state_offsets(padded_n);
        assert_eq!(hdr.weights_offset, (offsets.dendrite_weights + 128) as u32);
        assert_eq!(hdr.targets_offset, (offsets.dendrite_targets + 128) as u32);
        
        // Clean up
        let path = crate::utils::shm_file_path(zone_hash);
        let _ = std::fs::remove_file(path);
    }
}
