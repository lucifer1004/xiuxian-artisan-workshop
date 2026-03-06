//! Web ingestion primitives for Rust-native crawling and context extraction.

mod spider;
mod spider_config;

pub use spider::{SpiderBridge, WebContext};

#[cfg(test)]
mod tests;
