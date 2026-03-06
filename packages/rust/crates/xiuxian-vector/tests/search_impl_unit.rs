//! Integration harness for `search_impl` IPC conversion unit tests.

mod skill {
    pub use xiuxian_vector::skill::*;
}

mod search_impl_module {
    use xiuxian_vector::test_support::{
        keyword_boost, search_results_to_ipc, tool_search_results_to_ipc,
    };

    const _: fn() -> f32 = keyword_boost;

    mod tests;
}
