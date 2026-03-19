use std::collections::HashMap;

use super::super::types::{
    AnalysisEdge, AnalysisEdgeKind, AnalysisEvidence, AnalysisNode, AnalysisNodeKind,
};

pub(super) struct CompiledDocument {
    pub(super) document_hash: String,
    pub(super) nodes: Vec<AnalysisNode>,
    pub(super) edges: Vec<AnalysisEdge>,
    pub(super) diagnostics: Vec<String>,
}

pub(super) fn compile_markdown_ir(path: &str, content: &str) -> CompiledDocument {
    MarkdownCompiler::new(path, content).compile()
}

#[derive(Debug)]
struct EdgeDraft<'a> {
    kind: AnalysisEdgeKind,
    source_id: String,
    target_id: String,
    label: Option<String>,
    path: &'a str,
    line_start: usize,
    line_end: usize,
    confidence: f64,
}

struct MarkdownCompiler<'a> {
    path: &'a str,
    content: &'a str,
    nodes: Vec<AnalysisNode>,
    edges: Vec<AnalysisEdge>,
    diagnostics: Vec<String>,
    section_stack: Vec<(usize, String)>,
    task_chain: HashMap<String, String>,
    reference_nodes: HashMap<String, String>,
    edge_seq: usize,
    open_code_node_index: Option<usize>,
}

impl<'a> MarkdownCompiler<'a> {
    fn new(path: &'a str, content: &'a str) -> Self {
        Self {
            path,
            content,
            nodes: vec![AnalysisNode {
                id: "doc:0".to_string(),
                kind: AnalysisNodeKind::Document,
                label: path.to_string(),
                depth: 0,
                line_start: 1,
                line_end: content.lines().count().max(1),
                parent_id: None,
            }],
            edges: Vec::new(),
            diagnostics: Vec::new(),
            section_stack: Vec::new(),
            task_chain: HashMap::new(),
            reference_nodes: HashMap::new(),
            edge_seq: 1,
            open_code_node_index: None,
        }
    }

    fn compile(mut self) -> CompiledDocument {
        for (idx, raw_line) in self.content.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = raw_line.trim();

            if self.handle_code_fence(line_no, trimmed) {
                continue;
            }

            let context_node_id = self.handle_structure_nodes(line_no, trimmed);
            self.handle_wiki_links(line_no, raw_line, context_node_id.as_str());
        }

        self.finalize_unclosed_code_block();

        CompiledDocument {
            document_hash: blake3::hash(self.content.as_bytes()).to_hex().to_string(),
            nodes: self.nodes,
            edges: self.edges,
            diagnostics: self.diagnostics,
        }
    }

    fn handle_code_fence(&mut self, line_no: usize, trimmed: &str) -> bool {
        if !trimmed.starts_with("```") {
            return false;
        }

        if let Some(open_index) = self.open_code_node_index.take() {
            self.nodes[open_index].line_end = line_no;
            return true;
        }

        let language = trimmed.trim_start_matches("```").trim();
        let label = if language.is_empty() {
            "code block".to_string()
        } else {
            format!("code block ({language})")
        };
        let node_id = format!("code:{line_no}");
        let parent_id = current_context_node(&self.section_stack).to_string();
        let depth = self.section_stack.last().map_or(1, |(level, _)| level + 1);
        self.nodes.push(AnalysisNode {
            id: node_id.clone(),
            kind: AnalysisNodeKind::CodeBlock,
            label,
            depth,
            line_start: line_no,
            line_end: line_no,
            parent_id: Some(parent_id.clone()),
        });
        self.open_code_node_index = Some(self.nodes.len() - 1);
        self.push_edge(EdgeDraft {
            kind: AnalysisEdgeKind::Contains,
            source_id: parent_id,
            target_id: node_id,
            label: Some("contains".to_string()),
            path: self.path,
            line_start: line_no,
            line_end: line_no,
            confidence: 1.0,
        });
        true
    }

    fn handle_structure_nodes(&mut self, line_no: usize, trimmed: &str) -> String {
        if let Some((level, title)) = parse_heading(trimmed) {
            while self
                .section_stack
                .last()
                .is_some_and(|(active_level, _)| *active_level >= level)
            {
                self.section_stack.pop();
            }

            let parent_id = current_context_node(&self.section_stack).to_string();
            let node_id = format!("sec:{line_no}");
            self.nodes.push(AnalysisNode {
                id: node_id.clone(),
                kind: AnalysisNodeKind::Section,
                label: title,
                depth: level,
                line_start: line_no,
                line_end: line_no,
                parent_id: Some(parent_id.clone()),
            });
            self.push_edge(EdgeDraft {
                kind: AnalysisEdgeKind::Contains,
                source_id: parent_id,
                target_id: node_id.clone(),
                label: Some("contains".to_string()),
                path: self.path,
                line_start: line_no,
                line_end: line_no,
                confidence: 1.0,
            });
            self.section_stack.push((level, node_id.clone()));
            return node_id;
        }

        if let Some(task_label) = parse_task(trimmed) {
            let parent_id = current_context_node(&self.section_stack).to_string();
            let node_id = format!("task:{line_no}");
            let depth = self.section_stack.last().map_or(1, |(level, _)| level + 1);
            self.nodes.push(AnalysisNode {
                id: node_id.clone(),
                kind: AnalysisNodeKind::Task,
                label: task_label,
                depth,
                line_start: line_no,
                line_end: line_no,
                parent_id: Some(parent_id.clone()),
            });
            self.push_edge(EdgeDraft {
                kind: AnalysisEdgeKind::Contains,
                source_id: parent_id.clone(),
                target_id: node_id.clone(),
                label: Some("contains".to_string()),
                path: self.path,
                line_start: line_no,
                line_end: line_no,
                confidence: 1.0,
            });

            if let Some(prev_task) = self.task_chain.get(parent_id.as_str()) {
                self.push_edge(EdgeDraft {
                    kind: AnalysisEdgeKind::NextStep,
                    source_id: prev_task.clone(),
                    target_id: node_id.clone(),
                    label: Some("next".to_string()),
                    path: self.path,
                    line_start: line_no,
                    line_end: line_no,
                    confidence: 0.9,
                });
            }
            self.task_chain.insert(parent_id, node_id.clone());
            return node_id;
        }

        current_context_node(&self.section_stack).to_string()
    }

    fn handle_wiki_links(&mut self, line_no: usize, raw_line: &str, context_node_id: &str) {
        for target in extract_wiki_links(raw_line) {
            let normalized = normalize_reference(target.as_str());
            let reference_id = self.reference_node_id(normalized.as_str(), line_no);
            self.push_edge(EdgeDraft {
                kind: AnalysisEdgeKind::References,
                source_id: context_node_id.to_string(),
                target_id: reference_id,
                label: Some(normalized),
                path: self.path,
                line_start: line_no,
                line_end: line_no,
                confidence: 0.85,
            });
        }
    }

    fn reference_node_id(&mut self, normalized: &str, line_no: usize) -> String {
        if let Some(existing) = self.reference_nodes.get(normalized) {
            return existing.clone();
        }

        let node_id = format!("ref:{}", slugify(normalized));
        self.nodes.push(AnalysisNode {
            id: node_id.clone(),
            kind: AnalysisNodeKind::Reference,
            label: normalized.to_string(),
            depth: 1,
            line_start: line_no,
            line_end: line_no,
            parent_id: None,
        });
        self.reference_nodes
            .insert(normalized.to_string(), node_id.clone());
        node_id
    }

    fn push_edge(&mut self, draft: EdgeDraft<'_>) {
        self.edges.push(make_edge(self.edge_seq, draft));
        self.edge_seq += 1;
    }

    fn finalize_unclosed_code_block(&mut self) {
        if let Some(open_index) = self.open_code_node_index {
            self.diagnostics
                .push("Unclosed fenced code block; lineEnd set to document end".to_string());
            self.nodes[open_index].line_end = self.content.lines().count().max(1);
        }
    }
}

fn make_edge(sequence: usize, draft: EdgeDraft<'_>) -> AnalysisEdge {
    AnalysisEdge {
        id: format!("edge:{sequence}"),
        kind: draft.kind,
        source_id: draft.source_id,
        target_id: draft.target_id,
        label: draft.label,
        evidence: AnalysisEvidence {
            path: draft.path.to_string(),
            line_start: draft.line_start,
            line_end: draft.line_end,
            confidence: draft.confidence,
        },
    }
}

fn current_context_node(section_stack: &[(usize, String)]) -> &str {
    section_stack
        .last()
        .map_or("doc:0", |(_, node_id)| node_id.as_str())
}

fn parse_heading(line: &str) -> Option<(usize, String)> {
    if line.is_empty() || !line.starts_with('#') {
        return None;
    }
    let level = line.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let title = line[level..].trim();
    if title.is_empty() {
        None
    } else {
        Some((level, title.to_string()))
    }
}

fn parse_task(line: &str) -> Option<String> {
    let item = line
        .strip_prefix("- [")
        .or_else(|| line.strip_prefix("* ["))?;
    let close_index = item.find(']')?;
    if close_index != 1 {
        return None;
    }
    let status = item.chars().next()?;
    if status != ' ' && status != 'x' && status != 'X' {
        return None;
    }
    let rest = item.get(close_index + 1..)?.trim_start();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

fn extract_wiki_links(line: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut cursor = 0usize;
    while let Some(start) = line[cursor..].find("[[") {
        let absolute_start = cursor + start + 2;
        let Some(end_rel) = line[absolute_start..].find("]]") else {
            break;
        };
        let absolute_end = absolute_start + end_rel;
        let candidate = line[absolute_start..absolute_end].trim();
        if !candidate.is_empty() {
            links.push(candidate.to_string());
        }
        cursor = absolute_end + 2;
    }
    links
}

fn normalize_reference(raw: &str) -> String {
    raw.trim().replace('\\', "/")
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '/' || ch == '-' || ch == '_' || ch == ' ' {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}
