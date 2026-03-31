use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn markdown_to_telegram_markdown_v2(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut rendered = String::new();
    let mut ordered_list_stack: Vec<usize> = Vec::new();
    let mut list_is_ordered_stack: Vec<bool> = Vec::new();
    let mut link_stack: Vec<String> = Vec::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Strong => rendered.push('*'),
                Tag::Emphasis => rendered.push('_'),
                Tag::Strikethrough => rendered.push('~'),
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                    rendered.push_str("```\n");
                }
                Tag::Link { dest_url, .. } => {
                    rendered.push('[');
                    link_stack.push(dest_url.into_string());
                }
                Tag::Heading { .. } => rendered.push('*'),
                Tag::List(start) => {
                    if let Some(start_number) = start {
                        ordered_list_stack.push(start_number as usize);
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
                                rendered.push_str(&format!("{current}\\. "));
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

pub(super) fn escape_markdown_v2_text(text: &str) -> String {
    text.chars()
        .fold(String::with_capacity(text.len()), |mut escaped, ch| {
            match ch {
                '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '='
                | '|' | '{' | '}' | '.' | '!' | '\\' => {
                    escaped.push('\\');
                    escaped.push(ch);
                }
                _ => escaped.push(ch),
            }
            escaped
        })
}

pub(super) fn escape_markdown_v2_code(text: &str) -> String {
    text.chars()
        .fold(String::with_capacity(text.len()), |mut escaped, ch| {
            if ch == '\\' || ch == '`' {
                escaped.push('\\');
            }
            escaped.push(ch);
            escaped
        })
}

pub(super) fn escape_markdown_v2_url(url: &str) -> String {
    url.chars()
        .fold(String::with_capacity(url.len()), |mut escaped, ch| {
            if ch == '\\' || ch == ')' {
                escaped.push('\\');
            }
            escaped.push(ch);
            escaped
        })
}

pub(super) fn trim_trailing_blank_lines(text: &mut String) {
    while text.ends_with("\n\n") {
        text.pop();
    }
}

pub(super) fn normalize_code_fence_language(kind: CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Indented => None,
        CodeBlockKind::Fenced(language) => {
            let normalized = language
                .trim()
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
                .collect::<String>();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        }
    }
}

pub(super) fn escape_html_text(text: &str) -> String {
    text.chars()
        .fold(String::with_capacity(text.len()), |mut escaped, ch| {
            match ch {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                _ => escaped.push(ch),
            }
            escaped
        })
}

pub(super) fn escape_html_attr(text: &str) -> String {
    text.chars()
        .fold(String::with_capacity(text.len()), |mut escaped, ch| {
            match ch {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&#39;"),
                _ => escaped.push(ch),
            }
            escaped
        })
}

#[cfg(test)]
mod tests {
    use pulldown_cmark::CodeBlockKind;

    use super::{escape_html_attr, escape_html_text, normalize_code_fence_language};

    #[test]
    fn normalize_code_fence_language_filters_unsafe_characters() {
        let normalized =
            normalize_code_fence_language(CodeBlockKind::Fenced("rust<script>".into()));
        assert_eq!(normalized.as_deref(), Some("rustscript"));
    }

    #[test]
    fn escape_html_helpers_escape_reserved_characters() {
        assert_eq!(escape_html_text("a<&>b"), "a&lt;&amp;&gt;b");
        assert_eq!(escape_html_attr("\"a'&"), "&quot;a&#39;&amp;");
    }
}
