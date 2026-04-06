mod projection;
#[cfg(test)]
mod tests;

pub use projection::{
    BuiltinRerankRuntimeProjection, resolve_builtin_rerank_runtime_projection_with_settings,
};
