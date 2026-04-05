mod search;

#[cfg(test)]
mod tests;

pub(crate) use search::{AttachmentSearchError, search_attachment_hits};
