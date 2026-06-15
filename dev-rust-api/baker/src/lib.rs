//! Baker — Axicor .axic archive compilation pipeline.

pub mod error;
pub mod validator;
pub mod pipeline;
pub mod serialization;

pub use pipeline::{bake, AxicArchive};
