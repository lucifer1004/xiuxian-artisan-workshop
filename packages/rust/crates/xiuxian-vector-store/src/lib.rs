//! Storage-backed vector surface for `xiuxian-vector`.
//!
//! This crate is the explicit heavy dependency boundary for Lance-backed
//! vector search and storage operations. Lightweight Arrow/DataFusion helpers
//! stay in `xiuxian-vector`; storage-bound callers should depend on this crate.

pub use xiuxian_vector::*;
