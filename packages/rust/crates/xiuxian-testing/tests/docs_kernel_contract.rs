//! Contract tests for the `xiuxian-testing` docs kernel.

use std::{fs, path::Path};

fn read_file(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) => panic!("failed to read {}: {error}", path.display()),
    }
}

#[test]
fn docs_kernel_files_exist_and_are_indexed() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs_root = manifest_dir.join("docs");

    let required_files = [
        "index.md",
        "01_core/101_contract_testing_kernel.md",
        "03_features/201_rulepack_specification.md",
        "03_features/202_multi_role_audit_integration.md",
        "05_research/301_research_tracker.md",
        "06_roadmap/401_contract_testing_program.md",
    ];

    for relative_path in required_files {
        let full_path = docs_root.join(relative_path);
        assert!(
            full_path.is_file(),
            "expected docs kernel file to exist: {}",
            full_path.display()
        );
    }

    let index = read_file(&docs_root.join("index.md"));
    let expected_links = [
        "01_core/101_contract_testing_kernel",
        "03_features/201_rulepack_specification",
        "03_features/202_multi_role_audit_integration",
        "05_research/301_research_tracker",
        "06_roadmap/401_contract_testing_program",
    ];

    for link in expected_links {
        assert!(
            index.contains(link),
            "expected docs index to reference {link}"
        );
    }
}

#[test]
fn research_tracker_covers_core_papers() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tracker = read_file(&manifest_dir.join("docs/05_research/301_research_tracker.md"));

    let required_markers = [
        "OpenAI for OpenAPI",
        "Generating OpenAPI Specifications from Online API Documentation with Large Language Models",
        "LlamaRestTest",
        "SATORI",
        "METAMON",
        "MINES",
        "Oracular Programming",
        "Rethinking Testing for LLM Applications",
        "Uncovering Systematic Failures of LLMs in Verifying Code Against Natural Language Specifications",
        "Following Dragons",
        "A Survey of Code Review Benchmarks and Evaluation Practices in Pre-LLM and LLM Era",
        "Software Architecture Meets LLMs",
    ];

    for marker in required_markers {
        assert!(
            tracker.contains(marker),
            "expected research tracker to include {marker}"
        );
    }
}

#[test]
fn multi_role_integration_page_covers_runtime_stack() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let page =
        read_file(&manifest_dir.join("docs/03_features/202_multi_role_audit_integration.md"));

    let required_markers = [
        "Qianhuan",
        "Qianji",
        "ZhenfaPipeline",
        "ThoughtAggregator",
        "ArtifactObserver",
        "CognitiveTraceRecord",
        "Wendao",
        "ContractReport",
    ];

    for marker in required_markers {
        assert!(
            page.contains(marker),
            "expected multi-role integration page to include {marker}"
        );
    }
}
