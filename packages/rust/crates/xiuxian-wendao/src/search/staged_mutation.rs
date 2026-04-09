use std::collections::BTreeSet;

const DELETE_PATH_FILTER_BATCH_SIZE: usize = 100;

#[must_use]
pub(crate) fn path_delete_filters(column: &str, paths: &BTreeSet<String>) -> Vec<String> {
    if paths.is_empty() {
        return Vec::new();
    }

    let escaped = paths
        .iter()
        .map(|path| format!("'{}'", path.replace('\'', "''")))
        .collect::<Vec<_>>();
    escaped
        .chunks(DELETE_PATH_FILTER_BATCH_SIZE)
        .map(|chunk| format!("{column} IN ({})", chunk.join(",")))
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/search/staged_mutation.rs"]
mod tests;
