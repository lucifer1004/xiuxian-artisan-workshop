use crate::analyzers::{DocsPlannerRankReasonCode, DocsPlannerWorksetQuotaHint};

#[test]
fn workset_quota_hint_roundtrip() {
    let value = DocsPlannerWorksetQuotaHint {
        target_floor_count: 2,
        target_ceiling_count: 4,
        within_target_band: true,
    };
    let encoded = serde_json::to_string(&value)
        .unwrap_or_else(|error| panic!("serialize quota hint: {error}"));
    let decoded: DocsPlannerWorksetQuotaHint = serde_json::from_str(&encoded)
        .unwrap_or_else(|error| panic!("deserialize quota hint: {error}"));
    assert_eq!(decoded, value);
}

#[test]
fn rank_reason_code_serializes_as_snake_case() {
    let encoded = serde_json::to_string(&DocsPlannerRankReasonCode::ReferencePageBonus)
        .unwrap_or_else(|error| panic!("serialize rank reason: {error}"));
    assert_eq!(encoded, "\"reference_page_bonus\"");
}
