//! External Modelica Repo Intelligence plugin for `xiuxian-wendao`.

xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

mod plugin;

pub use plugin::{ModelicaRepoIntelligencePlugin, register_into};
