use std::fmt::Write as _;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use super::escape::{escape_html_attr, escape_html_text, trim_trailing_blank_lines};
use super::options::telegram_markdown_options;

#[must_use]
pub fn markdown_to_telegram_html(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, telegram_markdown_options());
    let mut rendered = String::new();
    let mut ordered_list_stack: Vec<usize> = Vec::new();
    let mut list_is_ordered_stack: Vec<bool> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Strong | Tag::Heading { .. } => rendered.push_str("<b>"),
                Tag::Emphasis => rendered.push_str("<i>"),
                Tag::Strikethrough => rendered.push_str("<s>"),
                Tag::CodeBlock(_) => {
                    rendered.push_str("<pre><code>");
                }
                Tag::Link { dest_url, .. } => {
                    rendered.push_str("<a href=\"");
                    rendered.push_str(&escape_html_attr(dest_url.as_ref()));
                    rendered.push_str("\">");
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
                                let _ = write!(rendered, "{current}. ");
                                *current += 1;
                            } else {
                                rendered.push_str("• ");
                            }
                        }
                        _ => rendered.push_str("• "),
                    }
                }
                Tag::BlockQuote(_) => rendered.push_str("&gt; "),
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Strong => rendered.push_str("</b>"),
                TagEnd::Emphasis => rendered.push_str("</i>"),
                TagEnd::Strikethrough => rendered.push_str("</s>"),
                TagEnd::CodeBlock => {
                    rendered.push_str("</code></pre>\n\n");
                }
                TagEnd::Link => rendered.push_str("</a>"),
                TagEnd::Heading(_) => rendered.push_str("</b>\n\n"),
                TagEnd::Paragraph => rendered.push_str("\n\n"),
                TagEnd::List(_) => {
                    ordered_list_stack.pop();
                    list_is_ordered_stack.pop();
                    rendered.push('\n');
                }
                _ => {}
            },
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                rendered.push_str(&escape_html_text(text.as_ref()));
            }
            Event::Code(text) => {
                rendered.push_str("<code>");
                rendered.push_str(&escape_html_text(text.as_ref()));
                rendered.push_str("</code>");
            }
            Event::SoftBreak | Event::HardBreak => rendered.push('\n'),
            Event::Rule => rendered.push_str("\n----\n"),
            Event::TaskListMarker(checked) => {
                if checked {
                    rendered.push_str("[x] ");
                } else {
                    rendered.push_str("[ ] ");
                }
            }
            Event::FootnoteReference(name) => {
                rendered.push('[');
                rendered.push_str(&escape_html_text(name.as_ref()));
                rendered.push(']');
            }
            _ => {}
        }
    }

    trim_trailing_blank_lines(&mut rendered);
    if rendered.is_empty() {
        escape_html_text(markdown)
    } else {
        rendered
    }
}
