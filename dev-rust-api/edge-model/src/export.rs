//! Exporter for C headers and binary partitions.

use crate::error::EdgeError;
use crate::EdgeModel;
use std::path::Path;

/// Exports SRAM and Flash binary images along with corresponding C headers.
pub fn export_c_headers(model: &EdgeModel, out_dir: &Path) -> Result<(), EdgeError> {
    let hot_state_bin_path = out_dir.join("axicor_hot_state.bin");
    let static_topology_bin_path = out_dir.join("axicor_static_topology.bin");
    let hot_state_h_path = out_dir.join("axicor_hot_state.h");
    let static_topology_h_path = out_dir.join("axicor_static_topology.h");

    std::fs::write(&hot_state_bin_path, &model.sram_blob).map_err(EdgeError::IoError)?;
    std::fs::write(&static_topology_bin_path, &model.flash_blob).map_err(EdgeError::IoError)?;

    let hot_state_h_content = format!(
        "#ifndef AXICOR_HOT_STATE_H\n\
         #define AXICOR_HOT_STATE_H\n\n\
         #include <stdint.h>\n\n\
         #define AXICOR_HOT_STATE_SIZE {}\n\n\
         extern const uint8_t axicor_hot_state_bin_start[];\n\
         extern const uint8_t axicor_hot_state_bin_end[];\n\n\
         #endif // AXICOR_HOT_STATE_H\n",
        model.sram_blob.len()
    );
    std::fs::write(&hot_state_h_path, hot_state_h_content).map_err(EdgeError::IoError)?;

    let static_topology_h_content = format!(
        "#ifndef AXICOR_STATIC_TOPOLOGY_H\n\
         #define AXICOR_STATIC_TOPOLOGY_H\n\n\
         #include <stdint.h>\n\n\
         #define AXICOR_STATIC_TOPOLOGY_SIZE {}\n\n\
         extern const uint8_t axicor_static_topology_bin_start[];\n\
         extern const uint8_t axicor_static_topology_bin_end[];\n\n\
         #endif // AXICOR_STATIC_TOPOLOGY_H\n",
        model.flash_blob.len()
    );
    std::fs::write(&static_topology_h_path, static_topology_h_content).map_err(EdgeError::IoError)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EdgeModel;
    use tempfile::tempdir;

    #[test]
    fn test_export_c_headers_success() {
        let model = EdgeModel {
            sram_blob: vec![1, 2, 3, 4],
            flash_blob: vec![5, 6, 7, 8, 9, 10],
        };
        let dir = tempdir().unwrap();
        let res = export_c_headers(&model, dir.path());
        assert!(res.is_ok());

        let bin_sram = std::fs::read(dir.path().join("axicor_hot_state.bin")).unwrap();
        assert_eq!(bin_sram, vec![1, 2, 3, 4]);

        let bin_flash = std::fs::read(dir.path().join("axicor_static_topology.bin")).unwrap();
        assert_eq!(bin_flash, vec![5, 6, 7, 8, 9, 10]);

        let h_sram = std::fs::read_to_string(dir.path().join("axicor_hot_state.h")).unwrap();
        assert!(h_sram.contains("#define AXICOR_HOT_STATE_SIZE 4"));

        let h_flash = std::fs::read_to_string(dir.path().join("axicor_static_topology.h")).unwrap();
        assert!(h_flash.contains("#define AXICOR_STATIC_TOPOLOGY_SIZE 6"));
    }

    #[test]
    fn test_export_c_headers_io_error() {
        let model = EdgeModel {
            sram_blob: vec![1],
            flash_blob: vec![2],
        };
        // Use a path that is guaranteed invalid / not writable
        let bad_path = std::path::Path::new("Z:\\nonexistent_directory_axicor_123");
        let res = export_c_headers(&model, bad_path);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), EdgeError::IoError(_)));
    }
}

