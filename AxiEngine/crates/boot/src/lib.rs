//! Crate `boot` provides loading and validation algorithms for single-shard `.axic` packages.

pub mod constants;
pub mod dto;
pub mod error;
pub mod loader;

pub use constants::{
    AXONS_ARCHIVE_PATH, PATHS_ARCHIVE_PATH, STATE_ARCHIVE_PATH, VARIANT_TABLE_ARCHIVE_PATH,
};
pub use dto::{LocalShardBootBundle, LocalShardBootInput, LocalShardComputeInput};
pub use error::BootError;
pub use loader::{bootstrap_local_shard_engine, load_local_shard_archive};
pub use compute::BackendPreference;
