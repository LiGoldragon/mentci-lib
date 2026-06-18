//! Psyche approval flow.
//!
//! Criome escalation asks the psyche a question. mentci holds that
//! question as typed state, presents it through a shell, and returns a
//! typed answer for the runtime to submit back to criome.

/// Local identifier for one approval question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApprovalIdentifier(u64);

impl ApprovalIdentifier {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Where an approval question came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalSource {
    /// Criome contract evaluation reached the human adjudicator rung.
    CriomeEscalation,
    /// A long-lived agent asked the psyche through the approval surface.
    AgentQuestion,
    /// A local system surface needs an explicit yes/no from the psyche.
    LocalSystemPrompt,
}

/// The question shown to the psyche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalPrompt(String);

impl ApprovalPrompt {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Suggested answer supplied by the asking agent or contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestedAnswer(String);

impl SuggestedAnswer {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Explanation for why the suggested answer is reasonable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalExplanation(String);

impl ApprovalExplanation {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Context the psyche needs before deciding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalContext(String);

impl ApprovalContext {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A pending approval question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalQuestion {
    pub identifier: ApprovalIdentifier,
    pub source: ApprovalSource,
    pub prompt: ApprovalPrompt,
    pub suggested_answer: SuggestedAnswer,
    pub explanation: ApprovalExplanation,
    pub context: ApprovalContext,
}

impl ApprovalQuestion {
    pub fn new(
        identifier: ApprovalIdentifier,
        source: ApprovalSource,
        prompt: ApprovalPrompt,
        suggested_answer: SuggestedAnswer,
        explanation: ApprovalExplanation,
        context: ApprovalContext,
    ) -> Self {
        Self {
            identifier,
            source,
            prompt,
            suggested_answer,
            explanation,
            context,
        }
    }
}

/// Psyche-authored answer text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalAnswer(String);

impl ApprovalAnswer {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Decision returned by the psyche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Accept the suggested answer exactly as proposed.
    ApproveSuggestedAnswer,
    /// Reject the proposal.
    Reject,
    /// Return a different answer.
    Answer(ApprovalAnswer),
    /// Leave this question pending for later.
    Defer,
}

/// Response ready for submission back to criome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalResponse {
    pub identifier: ApprovalIdentifier,
    pub decision: ApprovalDecision,
}

impl ApprovalResponse {
    pub fn new(identifier: ApprovalIdentifier, decision: ApprovalDecision) -> Self {
        Self {
            identifier,
            decision,
        }
    }
}

/// Approval state owned by the workbench model.
#[derive(Debug, Default, Clone)]
pub struct ApprovalState {
    pending: Vec<ApprovalQuestion>,
    selected: Option<ApprovalIdentifier>,
    answered: Vec<ApprovalResponse>,
}

impl ApprovalState {
    pub fn receive(&mut self, question: ApprovalQuestion) {
        let identifier = question.identifier;
        self.pending.push(question);
        if self.selected.is_none() {
            self.selected = Some(identifier);
        }
    }

    pub fn select(&mut self, identifier: ApprovalIdentifier) -> bool {
        if self
            .pending
            .iter()
            .any(|question| question.identifier == identifier)
        {
            self.selected = Some(identifier);
            true
        } else {
            false
        }
    }

    pub fn answer(&mut self, response: ApprovalResponse) -> Option<ApprovalQuestion> {
        if response.decision == ApprovalDecision::Defer {
            self.select(response.identifier);
            return None;
        }

        let index = self
            .pending
            .iter()
            .position(|question| question.identifier == response.identifier)?;
        let question = self.pending.remove(index);
        self.answered.push(response);
        self.selected = self
            .pending
            .get(index)
            .or_else(|| self.pending.last())
            .map(|next| next.identifier);
        Some(question)
    }

    pub fn current(&self) -> Option<&ApprovalQuestion> {
        let selected = self.selected?;
        self.pending
            .iter()
            .find(|question| question.identifier == selected)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn answered_count(&self) -> usize {
        self.answered.len()
    }

    pub fn view(&self) -> ApprovalView {
        ApprovalView {
            current: self.current().cloned(),
            pending_count: self.pending_count(),
            answered_count: self.answered_count(),
        }
    }
}

/// Pure-data approval snapshot for shells to paint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalView {
    pub current: Option<ApprovalQuestion>,
    pub pending_count: usize,
    pub answered_count: usize,
}
