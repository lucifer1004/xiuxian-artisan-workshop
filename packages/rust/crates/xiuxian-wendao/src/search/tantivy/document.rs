/// One shared search document stored in Tantivy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    /// Stable identifier used to map search hits back into domain records.
    pub id: String,
    /// Primary title or symbol name.
    pub title: String,
    /// Domain-specific kind label.
    pub kind: String,
    /// Stable path or location for the record.
    pub path: String,
    /// Coarse search scope such as repo or source.
    pub scope: String,
    /// Secondary namespace such as crate or document identifier.
    pub namespace: String,
    /// Additional searchable terms.
    pub terms: Vec<String>,
}
