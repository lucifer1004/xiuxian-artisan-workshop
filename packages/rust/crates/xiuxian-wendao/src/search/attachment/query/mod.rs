mod search;

#[cfg(test)]
#[path = "../../../../tests/unit/search/attachment/query/mod.rs"]
mod tests;

pub(crate) use search::{AttachmentSearchError, search_attachment_hits};
