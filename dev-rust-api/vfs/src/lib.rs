//! Virtual File System (VFS) for `.axic` archives.
//! Provides OS page aligned, zero-copy random access to compiled biological simulation parameters.

pub mod archive;
pub mod constants;
pub mod error;

pub use archive::*;
pub use constants::*;
pub use error::*;
