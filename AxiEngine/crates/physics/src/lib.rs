#![no_std]

//! Core domain mathematics and physics primitives for AxiEngine.
//!
//! This crate implements pure integer physics algorithms including GLIF membrane dynamics,
//! DDS spontaneous heartbeat generation, signal propagation with magnetic sentinel logic,
//! and GSOP synaptic plasticity adhering strictly to Dale's Law.

pub mod aot;
pub mod axon;
pub mod constants;
pub mod error;
pub mod glif;
pub mod gsop;

pub use aot::{compile_dds_heartbeat, compute_v_seg};
pub use axon::{active_tail_hit, initial_axon_head, propagate_head};
pub use constants::*;
pub use error::PhysicsError;
pub use glif::{heartbeat_spike, homeostasis_decay, is_glif_spike, update_glif_voltage};
pub use gsop::{apply_gsop_plasticity, inertia_rank, weight_to_charge};
