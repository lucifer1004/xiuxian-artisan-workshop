/// Axum runtime path for the legacy neighbors endpoint.
pub const API_NEIGHBORS_AXUM_PATH: &str = "/api/neighbors/{*id}";
/// `OpenAPI` path for the legacy neighbors endpoint.
pub const API_NEIGHBORS_OPENAPI_PATH: &str = "/api/neighbors/{id}";
/// Axum runtime path for the graph neighbors endpoint.
pub const API_GRAPH_NEIGHBORS_AXUM_PATH: &str = "/api/graph/neighbors/{*id}";
/// `OpenAPI` path for the graph neighbors endpoint.
pub const API_GRAPH_NEIGHBORS_OPENAPI_PATH: &str = "/api/graph/neighbors/{id}";
/// Axum runtime path for the 3D topology endpoint.
pub const API_TOPOLOGY_3D_AXUM_PATH: &str = "/api/topology/3d";
/// `OpenAPI` path for the 3D topology endpoint.
pub const API_TOPOLOGY_3D_OPENAPI_PATH: &str = "/api/topology/3d";
