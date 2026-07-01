//! Virtual File System (VFS) Crate for AxiEngine.
//!
//! Provides the binary layout specification, memory-mapped read-only parsing,
//! and deterministic serialization of the `.axic` archive containers.

#![deny(missing_docs)]

mod archive;
mod constants;
mod error;
mod packer;
mod path;

pub use archive::AxicArchive;
pub use error::VfsError;
pub use packer::{pack_entries, ArchiveEntry};
pub use path::validate_archive_path;
