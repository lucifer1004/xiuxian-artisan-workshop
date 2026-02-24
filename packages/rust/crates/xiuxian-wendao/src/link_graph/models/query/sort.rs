use super::enums::{LinkGraphSortField, LinkGraphSortOrder};
use serde::{Deserialize, Serialize};

/// One sort term (field + order).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkGraphSortTerm {
    /// Sort field.
    pub field: LinkGraphSortField,
    /// Sort order for the field.
    pub order: LinkGraphSortOrder,
}

impl Default for LinkGraphSortTerm {
    fn default() -> Self {
        Self {
            field: LinkGraphSortField::Score,
            order: LinkGraphSortOrder::Desc,
        }
    }
}
