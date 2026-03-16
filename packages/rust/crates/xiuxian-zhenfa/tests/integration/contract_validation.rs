//! XSD-backed contract validation coverage for qianji audit plans.

use std::path::{Path, PathBuf};

use xiuxian_zhenfa::{ZhenfaContractError, validate_contract, validate_contract_reference};

const VALID_AUDIT_PLAN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<qianji-audit-plan version="1.0">
  <summary>
    <intent>Validate the autonomous audit blueprint contract.</intent>
    <total-steps>1</total-steps>
  </summary>
  <implementation-steps>
    <step number="1">
      <title>Wire contract validation</title>
      <file-target path="packages/rust/crates/xiuxian-zhenfa/src/contracts/validation.rs" action="modify"/>
      <rationale>Qianji planner output must satisfy a strict contract.</rationale>
      <content>
        <description>Add schema validation for planner XML output.</description>
      </content>
    </step>
  </implementation-steps>
  <risk-assessment>
    <risk-pair severity="medium">
      <potential-issue>Validator rejects malformed plans too late.</potential-issue>
      <mitigation-strategy>Validate before the executor consumes planner output.</mitigation-strategy>
    </risk-pair>
  </risk-assessment>
  <verification-strategy>
    <test-command>cargo test -p xiuxian-zhenfa --test integration_test</test-command>
    <expected-outcome>Contract validation accepts the canonical audit plan.</expected-outcome>
  </verification-strategy>
</qianji-audit-plan>
"#;

const INVALID_AUDIT_PLAN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<qianji-audit-plan version="1.0">
  <summary>
    <intent>Break the schema on purpose.</intent>
    <total-steps>1</total-steps>
  </summary>
  <implementation-steps>
    <step number="1">
      <title>Use an unsupported action</title>
      <file-target path="packages/rust/crates/xiuxian-zhenfa/src/contracts/validation.rs" action="ship"/>
      <rationale>Schema must reject unsupported file actions.</rationale>
      <content>
        <description>This plan is intentionally invalid.</description>
      </content>
    </step>
  </implementation-steps>
  <risk-assessment>
    <risk-pair severity="medium">
      <potential-issue>Unsupported actions bypass contract checks.</potential-issue>
      <mitigation-strategy>Enforce XSD enumerations.</mitigation-strategy>
    </risk-pair>
  </risk-assessment>
  <verification-strategy>
    <test-command>cargo test</test-command>
    <expected-outcome>Schema validation must fail.</expected-outcome>
  </verification-strategy>
</qianji-audit-plan>
"#;

fn audit_contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| panic!("xiuxian-zhenfa should have a sibling crates directory"))
        .join("xiuxian-qianji")
        .join("src")
        .join("scenarios")
        .join("audit")
}

#[test]
fn validate_contract_accepts_qianji_audit_plan_schema() {
    let resolved =
        validate_contract_reference(VALID_AUDIT_PLAN, "qianji_plan.xsd", audit_contract_dir())
            .unwrap_or_else(|error| {
                panic!("valid audit plan should satisfy qianji_plan.xsd: {error}")
            });

    assert!(resolved.ends_with("qianji_plan.xsd"));
}

#[test]
fn validate_contract_rejects_schema_violations() {
    let Err(error) =
        validate_contract_reference(INVALID_AUDIT_PLAN, "qianji_plan.xsd", audit_contract_dir())
    else {
        panic!("invalid audit plan must fail schema validation");
    };

    assert!(matches!(
        error,
        ZhenfaContractError::ContractValidationFailed { .. }
    ));
}

#[test]
fn validate_contract_rejects_missing_contract_files() {
    let missing_contract = audit_contract_dir().join("missing_contract.xsd");
    let Err(error) = validate_contract(VALID_AUDIT_PLAN, &missing_contract) else {
        panic!("missing contract file must fail");
    };

    assert!(matches!(
        error,
        ZhenfaContractError::ContractNotFound { .. }
    ));
}
