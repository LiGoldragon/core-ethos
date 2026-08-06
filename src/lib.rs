#![allow(clippy::clone_on_copy)] // Legacy generic call sites intentionally accept opaque names.

//! # core-ethos
//!
//! Strict bootstrap Ethos reading for the Protos language family.
//!
//! [`bootstrap`] is the complete current boundary: a two-phase reader discovers
//! authority work, then seals the resulting meaning into a transaction whose
//! identity provenance can be validated before any lowering occurs.

pub mod bootstrap;
