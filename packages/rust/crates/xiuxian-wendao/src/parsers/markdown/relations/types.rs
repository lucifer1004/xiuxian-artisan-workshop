use crate::entity::RelationType;
use crate::link_graph::addressing::Address;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplicitRelationSource {
    pub heading_path: String,
    pub explicit_id: Option<String>,
}

impl ExplicitRelationSource {
    #[must_use]
    pub fn scope_address(&self) -> Option<Address> {
        if let Some(id) = &self.explicit_id {
            return Some(Address::id(id.clone()));
        }

        if self.heading_path.trim().is_empty() {
            return None;
        }

        Some(Address::path(
            self.heading_path
                .split(" / ")
                .map(str::trim)
                .filter(|component| !component.is_empty()),
        ))
    }

    #[must_use]
    pub fn scope_display(&self) -> Option<String> {
        self.scope_address()
            .map(|address| address.to_display_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplicitRelationTarget {
    pub note_target: Option<String>,
    pub address: Option<Address>,
    pub original: String,
}

impl ExplicitRelationTarget {
    #[must_use]
    pub fn display(&self) -> String {
        match (&self.note_target, &self.address) {
            (Some(note_target), Some(Address::Id(id))) => format!("{note_target}#{id}"),
            (Some(note_target), Some(Address::Hash(hash))) => format!("{note_target}@{hash}"),
            (Some(note_target), Some(address)) => {
                format!("{note_target}#{}", address.to_display_string())
            }
            (Some(note_target), None) => note_target.clone(),
            (None, Some(address)) => address.to_display_string(),
            (None, None) => self.original.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplicitSectionRelation {
    pub property_key: String,
    pub relation_type: RelationType,
    pub source: ExplicitRelationSource,
    pub target: ExplicitRelationTarget,
}
