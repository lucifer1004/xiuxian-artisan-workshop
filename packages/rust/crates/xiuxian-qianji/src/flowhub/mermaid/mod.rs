//! Mermaid parsing and validation for Flowhub scenario-case graphs.

mod model;
mod parse;
mod validate;

pub(crate) use model::{MermaidFlowchart, MermaidNodeKind};
pub(crate) use parse::parse_mermaid_flowchart;
pub(crate) use validate::validate_mermaid_flowchart;
