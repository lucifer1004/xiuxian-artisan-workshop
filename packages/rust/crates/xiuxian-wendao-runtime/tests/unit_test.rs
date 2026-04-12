//! Root unit-test harness for `xiuxian-wendao-runtime`.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/artifacts_openapi.rs"]
mod artifacts_openapi;

#[path = "unit/artifacts_zhixing.rs"]
mod artifacts_zhixing;
