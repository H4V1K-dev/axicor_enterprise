//! Platform-independent shared memory (SHM) infrastructure.

pub mod error;
pub mod constants;
pub mod utils;
pub mod platform;
pub mod shm;
pub mod sockets;
pub mod shadow;
pub mod ephys;
pub mod manifest;
pub mod mock;

pub use error::IpcError;
pub use constants::*;
pub use utils::*;
pub use shm::*;
pub use sockets::{BakerClient, BakerServer};
pub use shadow::ShadowShmManager;
pub use ephys::EphysManager;
pub use manifest::ManifestShmExporter;
pub use mock::MockShmAllocator;
