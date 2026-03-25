use std::collections::BTreeSet;

use crate::gateway::openapi::paths::{
    API_HEALTH_OPENAPI_PATH, API_NOTIFY_OPENAPI_PATH, API_REPO_SYNC_OPENAPI_PATH,
    WENDAO_GATEWAY_ROUTE_CONTRACTS,
};

#[test]
fn route_inventory_keeps_core_endpoints() {
    let openapi_paths = WENDAO_GATEWAY_ROUTE_CONTRACTS
        .iter()
        .map(|route| route.openapi_path)
        .collect::<BTreeSet<_>>();

    assert!(openapi_paths.contains(API_HEALTH_OPENAPI_PATH));
    assert!(openapi_paths.contains(API_NOTIFY_OPENAPI_PATH));
    assert!(openapi_paths.contains(API_REPO_SYNC_OPENAPI_PATH));
}

#[test]
fn route_inventory_paths_are_unique() {
    let openapi_paths = WENDAO_GATEWAY_ROUTE_CONTRACTS
        .iter()
        .map(|route| route.openapi_path)
        .collect::<BTreeSet<_>>();

    assert_eq!(openapi_paths.len(), WENDAO_GATEWAY_ROUTE_CONTRACTS.len());
}
