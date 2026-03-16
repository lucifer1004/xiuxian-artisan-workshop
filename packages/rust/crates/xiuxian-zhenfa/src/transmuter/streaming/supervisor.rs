//! Cognitive Supervisor: Thought Dimension Analysis (V3.0).
//!
//! This module categorizes streaming events into cognitive dimensions,
//! distinguishing between Meta-level (planning, self-reflection),
//! Operational-level (task execution, implementation), and
//! Epistemic-level (uncertainty, knowledge gaps) thoughts.
//!
//! ## V3.0 Features
//!
//! - **Three-Dimensional Cognitive Model**: MetaCognitive, Operational, Epistemic
//! - **Coherence Score**: Real-time quality assessment for Early-Halt
//! - **Hallucination Defense**: Second line of defense after LogicGate XSD validation
//! - **VecDeque History**: O(1) front removal for bounded history

use super::ZhenfaStreamingEvent;
use std::collections::VecDeque;

/// Maximum history size for cognitive tracking.
const MAX_HISTORY_SIZE: usize = 100;

/// Cognitive dimension classification for agent thoughts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CognitiveDimension {
    /// Meta-level cognition: planning, self-reflection, strategy evaluation.
    Meta,
    /// Operational-level cognition: task execution, implementation reasoning.
    Operational,
    /// Epistemic-level cognition: uncertainty, knowledge gaps, confidence assessment.
    Epistemic,
    /// System-level events: status, progress, errors.
    System,
    /// Tool interaction: calls and results.
    Instrumental,
}

/// Analyzed cognitive event with dimension classification.
#[derive(Debug, Clone, PartialEq)]
pub struct CognitiveEvent {
    /// The original streaming event.
    pub source: ZhenfaStreamingEvent,
    /// Classified cognitive dimension.
    pub dimension: CognitiveDimension,
    /// Optional sub-category for finer granularity.
    pub subcategory: Option<ThoughtSubcategory>,
    /// Confidence score for the classification (0.0-1.0).
    pub confidence: f32,
    /// Coherence score for this event (V2.0).
    pub coherence: f32,
}

/// Fine-grained subcategory for thought events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThoughtSubcategory {
    // Meta subcategories
    /// Planning future actions.
    Planning,
    /// Self-reflection on current approach.
    SelfReflection,
    /// Evaluating alternative strategies.
    StrategyEvaluation,
    /// Analyzing past errors.
    ErrorAnalysis,
    /// Setting or adjusting goals.
    GoalSetting,

    // Operational subcategories
    /// Analyzing code or file structure.
    CodeAnalysis,
    /// Reasoning about implementation details.
    ImplementationReasoning,
    /// Debugging active problems.
    Debugging,
    /// Reviewing or validating changes.
    Validation,
    /// Searching or exploring codebase.
    Exploration,

    // Epistemic subcategories (V2.0)
    /// Expressing uncertainty about approach.
    Uncertainty,
    /// Identifying knowledge gaps.
    KnowledgeGap,
    /// Seeking clarification or additional context.
    ClarificationSeeking,
    /// Assessing confidence in current solution.
    ConfidenceAssessment,
}

/// Coherence metrics for Early-Halt decision making.
#[derive(Debug, Clone, Default)]
pub struct CoherenceMetrics {
    /// Running coherence score (0.0-1.0).
    pub score: f32,
    /// Number of incoherent events detected.
    pub incoherent_count: u32,
    /// Number of self-correction events.
    pub self_correction_count: u32,
    /// Whether Early-Halt threshold has been breached.
    pub early_halt_triggered: bool,
}

/// Pattern matcher for cognitive dimension detection.
#[derive(Debug, Default)]
pub struct CognitiveSupervisor {
    /// History of classified thoughts for context.
    thought_history: Vec<CognitiveDimension>,
    /// Current operational context.
    context: SupervisorContext,
    /// Coherence metrics for Early-Halt (V2.0).
    coherence: CoherenceMetrics,
    /// Threshold for triggering Early-Halt.
    early_halt_threshold: f32,
}

/// Operational context for more accurate classification.
#[derive(Debug, Clone, Default)]
pub struct SupervisorContext {
    /// Whether the agent is in an active implementation phase.
    pub in_implementation: bool,
    /// Whether the agent has encountered recent errors.
    pub has_recent_errors: bool,
    /// Number of consecutive meta thoughts.
    pub meta_streak: u32,
    /// Number of consecutive operational thoughts.
    pub operational_streak: u32,
    /// Number of consecutive epistemic thoughts (V2.0).
    pub epistemic_streak: u32,
    /// Number of consecutive uncertainty expressions (V2.0).
    pub uncertainty_streak: u32,
}

impl CognitiveSupervisor {
    /// Create a new cognitive supervisor with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cognitive supervisor with custom Early-Halt threshold.
    #[must_use]
    pub fn with_early_halt_threshold(threshold: f32) -> Self {
        Self {
            early_halt_threshold: threshold.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Classify a streaming event into its cognitive dimension.
    #[must_use]
    pub fn classify(&mut self, event: ZhenfaStreamingEvent) -> CognitiveEvent {
        let (dimension, subcategory, confidence) = self.analyze_event(&event);

        // Track error state
        if matches!(event, ZhenfaStreamingEvent::Error { .. }) {
            self.context.has_recent_errors = true;
        }

        // Update context based on classification
        self.update_context(dimension);

        // Calculate coherence score (V2.0)
        let coherence = self.calculate_coherence(dimension, subcategory);

        CognitiveEvent {
            source: event,
            dimension,
            subcategory,
            confidence,
            coherence,
        }
    }

    /// Get current coherence metrics.
    #[must_use]
    pub const fn coherence(&self) -> &CoherenceMetrics {
        &self.coherence
    }

    /// Check if Early-Halt should be triggered.
    #[must_use]
    pub const fn should_halt(&self) -> bool {
        self.coherence.early_halt_triggered
    }

    /// Calculate coherence score for an event.
    fn calculate_coherence(
        &mut self,
        dimension: CognitiveDimension,
        subcategory: Option<ThoughtSubcategory>,
    ) -> f32 {
        // Base coherence
        let mut coherence = 0.8;

        // Reduce coherence for epistemic uncertainty
        if dimension == CognitiveDimension::Epistemic {
            coherence -= 0.15;
            self.context.uncertainty_streak += 1;

            // Excessive uncertainty streak reduces coherence further
            if self.context.uncertainty_streak > 3 {
                coherence -= 0.1 * (self.context.uncertainty_streak - 3) as f32;
            }
        } else {
            self.context.uncertainty_streak = 0;
        }

        // Self-reflection indicates coherence (meta-awareness)
        if matches!(subcategory, Some(ThoughtSubcategory::SelfReflection)) {
            coherence += 0.1;
            self.coherence.self_correction_count += 1;
        }

        // Error context reduces coherence
        if self.context.has_recent_errors {
            coherence -= 0.1;
        }

        // Long meta streak without operational action
        if self.context.meta_streak > 5 {
            coherence -= 0.05 * (self.context.meta_streak - 5) as f32;
        }

        // Detect incoherent patterns (oscillating between meta/operational without progress)
        if self.is_oscillating() {
            coherence -= 0.15;
            self.coherence.incoherent_count += 1;
        }

        coherence = coherence.clamp(0.0, 1.0);
        self.coherence.score = coherence;

        // Check Early-Halt threshold
        if coherence < self.early_halt_threshold && self.early_halt_threshold > 0.0 {
            self.coherence.early_halt_triggered = true;
        }

        coherence
    }

    /// Detect oscillating behavior (rapid switching between dimensions).
    fn is_oscillating(&self) -> bool {
        if self.thought_history.len() < 4 {
            return false;
        }

        // Check last 4 entries for pattern: A-B-A-B
        let len = self.thought_history.len();
        let last_4: Vec<_> = self.thought_history[len - 4..].iter().collect();

        // Pattern: Meta-Op-Meta-Op or Op-Meta-Op-Meta
        matches!(
            (&last_4[0], &last_4[1], &last_4[2], &last_4[3]),
            (
                CognitiveDimension::Meta,
                CognitiveDimension::Operational,
                CognitiveDimension::Meta,
                CognitiveDimension::Operational
            ) | (
                CognitiveDimension::Operational,
                CognitiveDimension::Meta,
                CognitiveDimension::Operational,
                CognitiveDimension::Meta
            )
        )
    }

    /// Analyze an event and return (dimension, subcategory, confidence).
    fn analyze_event(
        &self,
        event: &ZhenfaStreamingEvent,
    ) -> (CognitiveDimension, Option<ThoughtSubcategory>, f32) {
        match event {
            ZhenfaStreamingEvent::Thought(text) => self.classify_thought(text),
            ZhenfaStreamingEvent::TextDelta(_) => (CognitiveDimension::Operational, None, 0.9),
            ZhenfaStreamingEvent::ToolCall { .. } => (CognitiveDimension::Instrumental, None, 1.0),
            ZhenfaStreamingEvent::ToolResult { .. } => {
                (CognitiveDimension::Instrumental, None, 1.0)
            }
            ZhenfaStreamingEvent::Status(_) => (CognitiveDimension::System, None, 1.0),
            ZhenfaStreamingEvent::Progress { .. } => (CognitiveDimension::System, None, 1.0),
            ZhenfaStreamingEvent::Finished(_) => (CognitiveDimension::System, None, 1.0),
            ZhenfaStreamingEvent::Error { .. } => (CognitiveDimension::System, None, 1.0),
        }
    }

    /// Classify a thought text into cognitive dimension.
    fn classify_thought(
        &self,
        text: &str,
    ) -> (CognitiveDimension, Option<ThoughtSubcategory>, f32) {
        let text_lower = text.to_lowercase();

        // Meta pattern detection
        let meta_patterns = [
            // Planning patterns
            ("i should", ThoughtSubcategory::Planning),
            ("let me plan", ThoughtSubcategory::Planning),
            ("first, i'll", ThoughtSubcategory::Planning),
            ("my approach", ThoughtSubcategory::Planning),
            ("the plan is", ThoughtSubcategory::Planning),
            // Self-reflection patterns
            (
                "wait, let me reconsider",
                ThoughtSubcategory::SelfReflection,
            ),
            ("actually, i think", ThoughtSubcategory::SelfReflection),
            ("on second thought", ThoughtSubcategory::SelfReflection),
            ("i realize that", ThoughtSubcategory::SelfReflection),
            ("let me think about", ThoughtSubcategory::SelfReflection),
            // Strategy evaluation patterns
            (
                "alternative approach",
                ThoughtSubcategory::StrategyEvaluation,
            ),
            ("another way", ThoughtSubcategory::StrategyEvaluation),
            ("better approach", ThoughtSubcategory::StrategyEvaluation),
            ("instead of", ThoughtSubcategory::StrategyEvaluation),
            // Error analysis patterns
            ("the error", ThoughtSubcategory::ErrorAnalysis),
            ("went wrong", ThoughtSubcategory::ErrorAnalysis),
            ("failed because", ThoughtSubcategory::ErrorAnalysis),
            ("the issue is", ThoughtSubcategory::ErrorAnalysis),
            // Goal setting patterns
            ("my goal", ThoughtSubcategory::GoalSetting),
            ("the objective", ThoughtSubcategory::GoalSetting),
            ("what i want to achieve", ThoughtSubcategory::GoalSetting),
        ];

        // Operational pattern detection
        let operational_patterns = [
            // Code analysis patterns
            ("this function", ThoughtSubcategory::CodeAnalysis),
            ("the code", ThoughtSubcategory::CodeAnalysis),
            ("this file", ThoughtSubcategory::CodeAnalysis),
            ("the implementation", ThoughtSubcategory::CodeAnalysis),
            ("this module", ThoughtSubcategory::CodeAnalysis),
            // Implementation reasoning patterns
            ("i'll add", ThoughtSubcategory::ImplementationReasoning),
            ("i'll modify", ThoughtSubcategory::ImplementationReasoning),
            (
                "i need to change",
                ThoughtSubcategory::ImplementationReasoning,
            ),
            ("the fix is", ThoughtSubcategory::ImplementationReasoning),
            // Debugging patterns
            ("debugging", ThoughtSubcategory::Debugging),
            ("trace the issue", ThoughtSubcategory::Debugging),
            ("the bug", ThoughtSubcategory::Debugging),
            // Validation patterns
            ("verify that", ThoughtSubcategory::Validation),
            ("check if", ThoughtSubcategory::Validation),
            ("ensure that", ThoughtSubcategory::Validation),
            ("confirm that", ThoughtSubcategory::Validation),
            // Exploration patterns
            ("let me search", ThoughtSubcategory::Exploration),
            ("looking for", ThoughtSubcategory::Exploration),
            ("i need to find", ThoughtSubcategory::Exploration),
            ("exploring", ThoughtSubcategory::Exploration),
        ];

        // Epistemic pattern detection (V2.0)
        let epistemic_patterns = [
            // Uncertainty patterns
            ("i'm not sure", ThoughtSubcategory::Uncertainty),
            ("i'm uncertain", ThoughtSubcategory::Uncertainty),
            ("i don't know", ThoughtSubcategory::Uncertainty),
            ("unclear", ThoughtSubcategory::Uncertainty),
            ("might be", ThoughtSubcategory::Uncertainty),
            // Knowledge gap patterns
            ("i need more context", ThoughtSubcategory::KnowledgeGap),
            ("i need to understand", ThoughtSubcategory::KnowledgeGap),
            ("not familiar with", ThoughtSubcategory::KnowledgeGap),
            ("i haven't seen", ThoughtSubcategory::KnowledgeGap),
            // Clarification seeking patterns
            ("let me clarify", ThoughtSubcategory::ClarificationSeeking),
            ("can you clarify", ThoughtSubcategory::ClarificationSeeking),
            ("i should ask", ThoughtSubcategory::ClarificationSeeking),
            // Confidence assessment patterns
            ("i'm confident", ThoughtSubcategory::ConfidenceAssessment),
            ("my confidence", ThoughtSubcategory::ConfidenceAssessment),
            ("fairly certain", ThoughtSubcategory::ConfidenceAssessment),
            ("pretty sure", ThoughtSubcategory::ConfidenceAssessment),
        ];

        // Score epistemic patterns first (V2.0)
        let mut best_epistemic: Option<(ThoughtSubcategory, f32)> = None;
        for (pattern, subcategory) in &epistemic_patterns {
            if text_lower.contains(pattern) {
                let confidence = 0.75 + (pattern.len() as f32 / text.len().max(1) as f32).min(0.2);
                match best_epistemic {
                    None => best_epistemic = Some((*subcategory, confidence)),
                    Some((_, existing_conf)) if confidence > existing_conf => {
                        best_epistemic = Some((*subcategory, confidence));
                    }
                    _ => {}
                }
            }
        }

        // If epistemic pattern matched strongly, return it
        if let Some((subcategory, confidence)) = best_epistemic {
            return (CognitiveDimension::Epistemic, Some(subcategory), confidence);
        }

        // Score meta patterns
        let mut best_meta: Option<(ThoughtSubcategory, f32)> = None;
        for (pattern, subcategory) in &meta_patterns {
            if text_lower.contains(pattern) {
                let confidence = 0.7 + (pattern.len() as f32 / text.len().max(1) as f32).min(0.25);
                match best_meta {
                    None => best_meta = Some((*subcategory, confidence)),
                    Some((_, existing_conf)) if confidence > existing_conf => {
                        best_meta = Some((*subcategory, confidence));
                    }
                    _ => {}
                }
            }
        }

        // Score operational patterns
        let mut best_operational: Option<(ThoughtSubcategory, f32)> = None;
        for (pattern, subcategory) in &operational_patterns {
            if text_lower.contains(pattern) {
                let confidence = 0.7 + (pattern.len() as f32 / text.len().max(1) as f32).min(0.25);
                match best_operational {
                    None => best_operational = Some((*subcategory, confidence)),
                    Some((_, existing_conf)) if confidence > existing_conf => {
                        best_operational = Some((*subcategory, confidence));
                    }
                    _ => {}
                }
            }
        }

        // Decide between meta and operational
        match (best_meta, best_operational) {
            (Some((meta_sub, meta_conf)), Some((op_sub, op_conf))) => {
                // Both matched - use context to break tie
                if meta_conf > op_conf {
                    (CognitiveDimension::Meta, Some(meta_sub), meta_conf)
                } else if op_conf > meta_conf {
                    (CognitiveDimension::Operational, Some(op_sub), op_conf)
                } else {
                    // Equal confidence - use context
                    if self.context.in_implementation {
                        (CognitiveDimension::Operational, Some(op_sub), op_conf)
                    } else {
                        (CognitiveDimension::Meta, Some(meta_sub), meta_conf)
                    }
                }
            }
            (Some((subcategory, confidence)), None) => {
                (CognitiveDimension::Meta, Some(subcategory), confidence)
            }
            (None, Some((subcategory, confidence))) => (
                CognitiveDimension::Operational,
                Some(subcategory),
                confidence,
            ),
            (None, None) => {
                // No pattern matched - use heuristics
                self.classify_by_heuristics(text)
            }
        }
    }

    /// Fallback classification using heuristics when no patterns match.
    fn classify_by_heuristics(
        &self,
        text: &str,
    ) -> (CognitiveDimension, Option<ThoughtSubcategory>, f32) {
        let text_lower = text.to_lowercase();

        // Check for question marks (often meta-reflection)
        let question_count = text.matches('?').count();

        // Check for first-person pronouns (meta-indicator)
        let first_person_indicators = text_lower.matches(" i ").count()
            + text_lower.matches(" i'm ").count()
            + text_lower.matches(" my ").count();

        // Check for action verbs (operational-indicator)
        let action_verbs = [
            "add",
            "remove",
            "modify",
            "change",
            "create",
            "delete",
            "implement",
            "fix",
            "update",
            "refactor",
            "write",
        ];
        let action_count = action_verbs
            .iter()
            .filter(|verb| text_lower.contains(*verb))
            .count();

        // Heuristic scoring
        let meta_score = question_count as f32 * 0.2 + first_person_indicators as f32 * 0.3;
        let operational_score = action_count as f32 * 0.25;

        if meta_score > operational_score {
            (CognitiveDimension::Meta, None, 0.5 + meta_score * 0.1)
        } else if operational_score > 0.0 {
            (
                CognitiveDimension::Operational,
                None,
                0.5 + operational_score * 0.1,
            )
        } else {
            // Default to operational with low confidence
            (CognitiveDimension::Operational, None, 0.4)
        }
    }

    /// Update the supervisor context based on classification.
    fn update_context(&mut self, dimension: CognitiveDimension) {
        match dimension {
            CognitiveDimension::Meta => {
                self.context.meta_streak += 1;
                self.context.operational_streak = 0;
                self.context.epistemic_streak = 0;
            }
            CognitiveDimension::Operational => {
                self.context.operational_streak += 1;
                self.context.meta_streak = 0;
                self.context.epistemic_streak = 0;
                self.context.in_implementation = true;
            }
            CognitiveDimension::Epistemic => {
                self.context.epistemic_streak += 1;
                self.context.meta_streak = 0;
                self.context.operational_streak = 0;
            }
            CognitiveDimension::System => {
                // System events don't change the streak
            }
            CognitiveDimension::Instrumental => {
                // Tool interactions indicate operational phase
                self.context.in_implementation = true;
            }
        }

        self.thought_history.push(dimension);

        // Keep history bounded
        if self.thought_history.len() > 100 {
            self.thought_history.remove(0);
        }
    }

    /// Get the current context.
    #[must_use]
    pub fn context(&self) -> &SupervisorContext {
        &self.context
    }

    /// Reset the supervisor state.
    pub fn reset(&mut self) {
        self.thought_history.clear();
        self.context = SupervisorContext::default();
        self.coherence = CoherenceMetrics::default();
    }

    /// Get the cognitive balance (ratio of meta to operational thoughts).
    #[must_use]
    pub fn cognitive_balance(&self) -> f32 {
        if self.thought_history.is_empty() {
            return 0.5;
        }

        let meta_count = self
            .thought_history
            .iter()
            .filter(|&&d| d == CognitiveDimension::Meta)
            .count();

        meta_count as f32 / self.thought_history.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn supervisor_classifies_planning_as_meta() {
        let mut supervisor = CognitiveSupervisor::new();
        let event =
            ZhenfaStreamingEvent::Thought(Arc::from("Let me plan my approach to this problem."));

        let classified = supervisor.classify(event);
        assert_eq!(classified.dimension, CognitiveDimension::Meta);
        assert_eq!(classified.subcategory, Some(ThoughtSubcategory::Planning));
    }

    #[test]
    fn supervisor_classifies_code_analysis_as_operational() {
        let mut supervisor = CognitiveSupervisor::new();
        let event =
            ZhenfaStreamingEvent::Thought(Arc::from("This function handles the validation logic."));

        let classified = supervisor.classify(event);
        assert_eq!(classified.dimension, CognitiveDimension::Operational);
        assert_eq!(
            classified.subcategory,
            Some(ThoughtSubcategory::CodeAnalysis)
        );
    }

    #[test]
    fn supervisor_classifies_tool_call_as_instrumental() {
        let mut supervisor = CognitiveSupervisor::new();
        let event = ZhenfaStreamingEvent::ToolCall {
            id: Arc::from("call_1"),
            name: Arc::from("read_file"),
            input: serde_json::Value::Null,
        };

        let classified = supervisor.classify(event);
        assert_eq!(classified.dimension, CognitiveDimension::Instrumental);
    }

    #[test]
    fn supervisor_tracks_cognitive_balance() {
        let mut supervisor = CognitiveSupervisor::new();

        // Add some meta thoughts
        let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from("Let me plan...")));
        let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from(
            "I should reconsider...",
        )));

        // Add some operational thoughts
        let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from(
            "This code needs...",
        )));
        let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from(
            "I'll add a function...",
        )));

        let balance = supervisor.cognitive_balance();
        assert!(balance > 0.0 && balance < 1.0);
    }

    #[test]
    fn supervisor_context_updates_on_classification() {
        let mut supervisor = CognitiveSupervisor::new();

        // Classify operational thought
        let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from(
            "I'll implement this feature.",
        )));

        assert!(supervisor.context().in_implementation);
        assert_eq!(supervisor.context().operational_streak, 1);
    }

    // V2.0 Tests

    #[test]
    fn supervisor_classifies_uncertainty_as_epistemic() {
        let mut supervisor = CognitiveSupervisor::new();
        let event =
            ZhenfaStreamingEvent::Thought(Arc::from("I'm not sure if this approach is correct."));

        let classified = supervisor.classify(event);
        assert_eq!(classified.dimension, CognitiveDimension::Epistemic);
        assert_eq!(
            classified.subcategory,
            Some(ThoughtSubcategory::Uncertainty)
        );
    }

    #[test]
    fn supervisor_classifies_knowledge_gap_as_epistemic() {
        let mut supervisor = CognitiveSupervisor::new();
        let event = ZhenfaStreamingEvent::Thought(Arc::from("I need more context about this API."));

        let classified = supervisor.classify(event);
        assert_eq!(classified.dimension, CognitiveDimension::Epistemic);
        assert_eq!(
            classified.subcategory,
            Some(ThoughtSubcategory::KnowledgeGap)
        );
    }

    #[test]
    fn supervisor_calculates_coherence_score() {
        let mut supervisor = CognitiveSupervisor::new();

        // Coherent operational thought
        let event = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from(
            "I'll add a function here.",
        )));
        assert!(event.coherence > 0.7);

        // Epistemic uncertainty reduces coherence
        let event = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from(
            "I'm not sure about this.",
        )));
        assert!(event.coherence < 0.8);
    }

    #[test]
    fn supervisor_triggers_early_halt_on_low_coherence() {
        let mut supervisor = CognitiveSupervisor::with_early_halt_threshold(0.5);

        // Trigger multiple uncertainty events
        for _ in 0..5 {
            let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from(
                "I'm not sure about this approach.",
            )));
        }

        // Check if early halt was triggered
        assert!(supervisor.should_halt());
    }

    #[test]
    fn supervisor_detects_oscillating_behavior() {
        let mut supervisor = CognitiveSupervisor::new();

        // Create oscillating pattern: Meta-Op-Meta-Op
        let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from("Let me plan...")));
        let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from("This function...")));
        let _ = supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from(
            "I should reconsider...",
        )));
        let event =
            supervisor.classify(ZhenfaStreamingEvent::Thought(Arc::from("I'll add code...")));

        // Oscillating behavior should reduce coherence
        assert!(event.coherence < 0.75);
    }
}
