//! Manifest exporter for topology metadata.

use crate::error::IpcError;

/// Exporter for compiling zone metadata and writing it atomically.
pub struct ManifestShmExporter;

impl ManifestShmExporter {
    /// Export the manifest byte payload atomically to the target file.
    ///
    /// # Invariants and Edge Cases
    /// - **R-007**: Manifest Read/Write Race protection.
    ///   To prevent the daemon from reading a partially written manifest file, the exporter
    ///   first writes the complete payload to a temporary file (`.tmp`) and then invokes
    ///   the atomic OS system call `std::fs::rename` to overwrite the final path.
    pub fn export(zone_hash: u32, payload: &[u8]) -> Result<(), IpcError> {
        let final_path = crate::utils::manifest_shm_path(zone_hash);
        
        let mut tmp_path = final_path.clone();
        if let Some(name) = final_path.file_name() {
            let mut tmp_name = name.to_os_string();
            tmp_name.push(".tmp");
            tmp_path.set_file_name(tmp_name);
        } else {
            return Err(IpcError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid manifest filename path",
            )));
        }

        // Write payload to temporary path first
        std::fs::write(&tmp_path, payload).map_err(IpcError::Io)?;

        // Atomically rename it to the final destination path
        std::fs::rename(&tmp_path, &final_path).map_err(IpcError::Io)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_atomic_export() {
        let zone_hash = 888;
        let payload = b"manifest_data_payload";
        
        ManifestShmExporter::export(zone_hash, payload).expect("Failed to export manifest");
        
        let final_path = crate::utils::manifest_shm_path(zone_hash);
        assert!(final_path.exists());
        
        let read_payload = std::fs::read(&final_path).expect("Failed to read manifest");
        assert_eq!(read_payload, payload);
        
        // Clean up
        let _ = std::fs::remove_file(final_path);
    }
}
