#[derive(Debug, Default)]
pub(in crate::parsers::markdown) struct ExtractedLinkTargets {
    pub note_links: Vec<String>,
    pub attachments: Vec<String>,
}

#[derive(Debug)]
pub(in crate::parsers::markdown) enum ParsedTarget {
    Note(String),
    Attachment(String),
}
