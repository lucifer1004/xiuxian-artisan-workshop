//! Canonical unit-test harness for `xiuxian-daochang`.
#![recursion_limit = "256"]

xiuxian_testing::crate_test_policy_harness!();

mod agent {
    pub(crate) use crate::unit::session_redis::agent::{
        Agent, SessionContextMode, SessionContextWindowInfo,
    };

    pub(crate) mod session_context {
        pub(crate) use xiuxian_daochang::test_support::now_unix_ms;
    }
}

mod observability {
    pub(crate) use crate::unit::session_redis::observability::SessionEvent;
}

mod session {
    pub(crate) use xiuxian_daochang::{ChatMessage, SessionSummarySegment};
}

mod unit;
