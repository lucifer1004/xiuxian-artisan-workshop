use std::fs;
use std::path::PathBuf;

fn query_adapter_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/search/queries")
}

fn adapter_tests_dir(adapter: &str) -> PathBuf {
    query_adapter_root().join(adapter).join("tests")
}

fn query_snapshot_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/search/queries")
}

fn legacy_studio_snapshot_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/gateway/studio")
}

fn expected_query_snapshots() -> [(&'static str, &'static [&'static str]); 4] {
    [
        (
            "sql",
            &[
                "sql_query_surface_payload.snap",
                "sql_discovery_surface_payload.snap",
            ],
        ),
        ("graphql", &["graphql_query_surface_payload.snap"]),
        ("flightsql", &["flightsql_query_surface_payload.snap"]),
        ("rest", &["rest_query_surface_payload.snap"]),
    ]
}

#[test]
fn canonical_query_adapters_must_keep_snapshot_modules() {
    for (adapter, snapshots) in expected_query_snapshots() {
        let tests_dir = adapter_tests_dir(adapter);
        let snapshots_path = tests_dir.join("snapshots.rs");
        assert!(
            snapshots_path.is_file(),
            "canonical query adapter `{adapter}` must keep `{}`",
            snapshots_path.display()
        );

        let mod_path = tests_dir.join("mod.rs");
        let mod_content = fs::read_to_string(&mod_path)
            .unwrap_or_else(|error| panic!("read {mod_path:?}: {error}"));
        assert!(
            mod_content.contains("mod snapshots;"),
            "canonical query adapter `{adapter}` must declare `mod snapshots;` in `{}`",
            mod_path.display()
        );

        for snapshot in snapshots {
            let baseline_path = query_snapshot_root().join(snapshot);
            assert!(
                baseline_path.is_file(),
                "canonical query adapter `{adapter}` must keep snapshot baseline `{}`",
                baseline_path.display()
            );

            let legacy_path = legacy_studio_snapshot_root().join(format!("studio_{snapshot}"));
            assert!(
                !legacy_path.exists(),
                "canonical query adapter `{adapter}` must not keep legacy Studio snapshot `{}`",
                legacy_path.display()
            );
        }
    }
}
