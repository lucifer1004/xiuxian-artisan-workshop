use std::fmt::Write as _;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use super::escape::{
    escape_markdown_v2_code, escape_markdown_v2_text, escape_markdown_v2_url,
    normalize_code_fence_language, trim_trailing_blank_lines,
};
use super::options::telegram_markdown_options;

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn markdown_to_telegram_markdown_v2(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, telegram_markdown_options());
    let mut rendered = String::new();
    let mut ordered_list_stack: Vec<usize> = Vec::new();
    let mut list_is_ordered_stack: Vec<bool> = Vec::new();
    let mut link_stack: Vec<String> = Vec::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Strong | Tag::Heading { .. } => rendered.push('*'),
                Tag::Emphasis => rendered.push('_'),
                Tag::Strikethrough => rendered.push('~'),
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    rendered.push_str("```");
                    if let Some(language) = normalize_code_fence_language(kind) {
                        rendered.push_str(&language);
                    }
                    rendered.push('\n');
                }
                Tag::Link { dest_url, .. } => {
                    rendered.push('[');
                    link_stack.push(dest_url.into_string());
                }
                Tag::List(start) => {
                    if let Some(start_number) = start {
                        ordered_list_stack
                            .push(usize::try_from(start_number).unwrap_or(usize::MAX));
                        list_is_ordered_stack.push(true);
                    } else {
                        ordered_list_stack.push(1);
                        list_is_ordered_stack.push(false);
                    }
                }
                Tag::Item => {
                    if !rendered.is_empty() && !rendered.ends_with('\n') {
                        rendered.push('\n');
                    }
                    match list_is_ordered_stack.last().copied() {
                        Some(true) => {
                            if let Some(current) = ordered_list_stack.last_mut() {
                                let _ = write!(rendered, "{current}\\. ");
                                *current += 1;
                            } else {
                                rendered.push_str("• ");
                            }
                        }
                        _ => rendered.push_str("• "),
                    }
                }
                Tag::BlockQuote(_) => rendered.push_str("> "),
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Strong => rendered.push('*'),
                TagEnd::Emphasis => rendered.push('_'),
                TagEnd::Strikethrough => rendered.push('~'),
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    if !rendered.ends_with('\n') {
                        rendered.push('\n');
                    }
                    rendered.push_str("```\n\n");
                }
                TagEnd::Link => {
                    let link_target = link_stack.pop().unwrap_or_default();
                    rendered.push(']');
                    rendered.push('(');
                    rendered.push_str(&escape_markdown_v2_url(&link_target));
                    rendered.push(')');
                }
                TagEnd::Heading(_) => rendered.push_str("*\n\n"),
                TagEnd::Paragraph => rendered.push_str("\n\n"),
                TagEnd::List(_) => {
                    ordered_list_stack.pop();
                    list_is_ordered_stack.pop();
                    rendered.push('\n');
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    rendered.push_str(&escape_markdown_v2_code(text.as_ref()));
                } else {
                    rendered.push_str(&escape_markdown_v2_text(text.as_ref()));
                }
            }
            Event::Code(text) => {
                rendered.push('`');
                rendered.push_str(&escape_markdown_v2_code(text.as_ref()));
                rendered.push('`');
            }
            Event::SoftBreak | Event::HardBreak => rendered.push('\n'),
            Event::Rule => rendered.push_str("\n\\-\\-\\-\\-\n"),
            Event::Html(text) | Event::InlineHtml(text) => {
                rendered.push_str(&escape_markdown_v2_text(text.as_ref()));
            }
            Event::TaskListMarker(checked) => {
                if checked {
                    rendered.push_str("\\[x\\] ");
                } else {
                    rendered.push_str("\\[ \\] ");
                }
            }
            Event::FootnoteReference(name) => {
                rendered.push_str("\\[");
                rendered.push_str(&escape_markdown_v2_text(name.as_ref()));
                rendered.push_str("\\]");
            }
            _ => {}
        }
    }

    trim_trailing_blank_lines(&mut rendered);
    if rendered.is_empty() {
        escape_markdown_v2_text(markdown)
    } else {
        rendered
    }
}
