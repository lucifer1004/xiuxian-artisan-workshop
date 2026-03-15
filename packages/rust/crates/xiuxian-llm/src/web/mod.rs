//! Web ingestion primitives for Rust-native crawling and context extraction.

mod spider;

pub use spider::{SpiderBridge, WebContext};

#[cfg(test)]
mod tests;
