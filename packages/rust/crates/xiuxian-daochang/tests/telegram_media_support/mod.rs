//! Test coverage for xiuxian-daochang behavior.

mod bootstrap;
pub(crate) mod media_api;
pub(crate) mod upload_api;

pub(crate) use media_api::{
    spawn_mock_telegram_media_api, spawn_mock_telegram_media_api_with_group_failure,
};
