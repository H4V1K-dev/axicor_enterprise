//! Helper functions to generate deterministic paths for shared memory and sockets.

use std::path::PathBuf;

/// Generates the path to the state shared memory file.
///
/// On Linux, this is strictly `/dev/shm/axicor_state_{zone_hash}.shm`.
/// On other platforms (e.g. Windows), it falls back to `%TEMP%/axicor_state_{zone_hash}.tmp`.
pub fn shm_file_path(zone_hash: u32) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!("{}/axicor_state_{}.shm", crate::constants::LINUX_SHM_DIR, zone_hash))
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::temp_dir().join(format!("axicor_state_{}.tmp", zone_hash))
    }
}

/// Generates the default socket or network connection endpoint path.
///
/// On Linux, this is strictly `/tmp/axicor_baker_{zone_hash}.sock`.
/// On other platforms (e.g. Windows), it falls back to a loopback TCP address: `127.0.0.1:{12000 + (zone_hash % 40000)}`.
pub fn default_socket_path(zone_hash: u32) -> String {
    #[cfg(target_os = "linux")]
    {
        format!("{}/axicor_baker_{}.sock", crate::constants::LINUX_UDS_DIR, zone_hash)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let port = 12000 + (zone_hash % 40000);
        format!("127.0.0.1:{}", port)
    }
}

/// Generates the path to the shadow shared memory file.
///
/// On Linux, this is strictly `/dev/shm/axicor_shadow_{zone_hash}_{suffix}.shm`.
/// On other platforms (e.g. Windows), it falls back to `%TEMP%/axicor_shadow_{zone_hash}_{suffix}.tmp`.
pub fn shadow_file_path(zone_hash: u32, suffix: &str) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!("{}/axicor_shadow_{}_{}.shm", crate::constants::LINUX_SHM_DIR, zone_hash, suffix))
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::temp_dir().join(format!("axicor_shadow_{}_{}.tmp", zone_hash, suffix))
    }
}

/// Generates the path to the electrophysiology shared memory file.
///
/// On Linux, this is strictly `/dev/shm/axicor_ephys_{zone_hash}.shm`.
/// On other platforms (e.g. Windows), it falls back to `%TEMP%/axicor_ephys_{zone_hash}.tmp`.
pub fn ephys_shm_path(zone_hash: u32) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!("{}/axicor_ephys_{}.shm", crate::constants::LINUX_SHM_DIR, zone_hash))
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::temp_dir().join(format!("axicor_ephys_{}.tmp", zone_hash))
    }
}

/// Generates the path to the manifest shared memory file.
///
/// On Linux, this is strictly `/dev/shm/axicor_manifest_{zone_hash}.bin`.
/// On other platforms (e.g. Windows), it falls back to `%TEMP%/axicor_manifest_{zone_hash}.tmp`.
pub fn manifest_shm_path(zone_hash: u32) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from(format!("{}/axicor_manifest_{}.bin", crate::constants::LINUX_SHM_DIR, zone_hash))
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::temp_dir().join(format!("axicor_manifest_{}.tmp", zone_hash))
    }
}
