/// Default Julia analyzer package directory relative to the repository root.
pub const DEFAULT_JULIA_ANALYZER_PACKAGE_DIR: &str = ".data/WendaoAnalyzer";

/// Default Julia analyzer launcher path used by Wendao compatibility surfaces.
pub const DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH: &str =
    ".data/WendaoAnalyzer/scripts/run_analyzer_service.sh";

/// Default Julia analyzer example config path used by Wendao compatibility surfaces.
pub const DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH: &str =
    ".data/WendaoAnalyzer/config/analyzer.example.toml";

/// Canonical Arrow Flight rerank route used by Julia compatibility surfaces.
pub const DEFAULT_JULIA_RERANK_FLIGHT_ROUTE: &str = "/rerank";
