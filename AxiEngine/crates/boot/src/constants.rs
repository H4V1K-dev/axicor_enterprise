//! Stable logical paths for single-shard archive files.

/// The archive path for the `.state` file.
pub const STATE_ARCHIVE_PATH: &str = "state.bin";

/// The archive path for the `.axons` file.
pub const AXONS_ARCHIVE_PATH: &str = "axons.bin";

/// The archive path for the `.paths` file.
pub const PATHS_ARCHIVE_PATH: &str = "paths.bin";

/// The archive path for the `VariantParameters` lookup table.
pub const VARIANT_TABLE_ARCHIVE_PATH: &str = "variant_table.bin";
