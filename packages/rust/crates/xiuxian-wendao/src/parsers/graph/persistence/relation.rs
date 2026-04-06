use crate::entity::RelationType;
use std::str::FromStr;

/// Parse the persisted `relation_type` field written by Wendao graph storage.
pub(super) fn parse_persisted_relation_type(raw: &str) -> RelationType {
    let persisted_token = raw.trim();

    match RelationType::from_str(persisted_token) {
        Ok(RelationType::Other(_)) => RelationType::Other(persisted_token.to_string()),
        Ok(relation_type) => relation_type,
        Err(never) => match never {},
    }
}
