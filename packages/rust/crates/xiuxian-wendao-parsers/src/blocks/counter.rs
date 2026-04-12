use super::MarkdownBlockKind;

#[derive(Default)]
pub(super) struct BlockIndexCounter {
    para: usize,
    code: usize,
    ulist: usize,
    olist: usize,
    quote: usize,
    hr: usize,
    table: usize,
    html: usize,
}

impl BlockIndexCounter {
    pub(super) fn next(&mut self, kind: &MarkdownBlockKind) -> usize {
        match kind {
            MarkdownBlockKind::Paragraph => {
                let idx = self.para;
                self.para += 1;
                idx
            }
            MarkdownBlockKind::CodeFence { .. } => {
                let idx = self.code;
                self.code += 1;
                idx
            }
            MarkdownBlockKind::List { ordered: true } => {
                let idx = self.olist;
                self.olist += 1;
                idx
            }
            MarkdownBlockKind::List { ordered: false } => {
                let idx = self.ulist;
                self.ulist += 1;
                idx
            }
            MarkdownBlockKind::BlockQuote => {
                let idx = self.quote;
                self.quote += 1;
                idx
            }
            MarkdownBlockKind::ThematicBreak => {
                let idx = self.hr;
                self.hr += 1;
                idx
            }
            MarkdownBlockKind::Table => {
                let idx = self.table;
                self.table += 1;
                idx
            }
            MarkdownBlockKind::HtmlBlock => {
                let idx = self.html;
                self.html += 1;
                idx
            }
        }
    }
}
