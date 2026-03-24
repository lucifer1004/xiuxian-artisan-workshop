use specta::TypeCollection;

/// Build the Studio Specta type collection used by `export_types`.
#[must_use]
pub fn studio_type_collection() -> TypeCollection {
    TypeCollection::default()
}
