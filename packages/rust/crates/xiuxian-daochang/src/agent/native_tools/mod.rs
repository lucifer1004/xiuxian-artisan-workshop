pub(crate) mod macros;
/// Built-in Rust native tools for the agent.
pub mod registry;
/// Spider-based web crawling native tools.
pub mod spider;
/// Zhixing-Heyi specific native tools.
pub mod zhixing;

pub use registry::NativeToolRegistry;
