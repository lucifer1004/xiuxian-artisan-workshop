pub use xiuxian_wendao_builtin::bootstrap_builtin_registry;

#[cfg(all(test, feature = "julia"))]
#[path = "../../../tests/unit/analyzers/service/bootstrap.rs"]
mod tests;
