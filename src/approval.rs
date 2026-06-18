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

/// Local identifier for a client subscribed to approval state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApprovalClientIdentifier(u64);

impl ApprovalClientIdentifier {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Local identifier for one approval-state subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApprovalSubscriptionIdentifier(u64);

impl ApprovalSubscriptionIdentifier {
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

/// Body of a psyche-authored answer proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerBody(String);

impl AnswerBody {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A different answer authored as a separate object.
///
/// This is not a verdict variant. The runtime submits it through the normal
/// authorization path, then the psyche can approve that object's digest with
/// the same closed verdict set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerProposal {
    pub question: ApprovalIdentifier,
    pub body: AnswerBody,
}

impl AnswerProposal {
    pub fn new(question: ApprovalIdentifier, body: AnswerBody) -> Self {
        Self { question, body }
    }
}

/// Decision returned by the psyche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Accept the suggested answer exactly as proposed.
    ApproveSuggestedAnswer,
    /// Reject the proposal.
    Reject,
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

/// Which approval-state updates a client wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalInterest {
    /// Every approval state update.
    All,
    /// New pending questions and selection changes.
    PendingQuestions,
    /// Completed approval responses.
    AnsweredResponses,
}

impl ApprovalInterest {
    fn accepts(&self, update: &ApprovalUpdate) -> bool {
        match self {
            Self::All => true,
            Self::PendingQuestions => matches!(
                update,
                ApprovalUpdate::Snapshot(_)
                    | ApprovalUpdate::QuestionReceived(_)
                    | ApprovalUpdate::QuestionSelected(_)
            ),
            Self::AnsweredResponses => matches!(
                update,
                ApprovalUpdate::Snapshot(_) | ApprovalUpdate::QuestionAnswered(_)
            ),
        }
    }
}

/// One client subscription over the daemon-owned approval state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalSubscription {
    pub identifier: ApprovalSubscriptionIdentifier,
    pub client: ApprovalClientIdentifier,
    pub interest: ApprovalInterest,
}

impl ApprovalSubscription {
    pub fn new(
        identifier: ApprovalSubscriptionIdentifier,
        client: ApprovalClientIdentifier,
        interest: ApprovalInterest,
    ) -> Self {
        Self {
            identifier,
            client,
            interest,
        }
    }

    fn delivery_for(&self, update: &ApprovalUpdate) -> Option<ApprovalDelivery> {
        if self.interest.accepts(update) {
            Some(ApprovalDelivery::new(
                self.identifier,
                self.client,
                update.clone(),
            ))
        } else {
            None
        }
    }
}

/// Receipt returned when a client subscribes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalSubscriptionReceipt {
    pub subscription: ApprovalSubscription,
    pub snapshot: ApprovalView,
}

impl ApprovalSubscriptionReceipt {
    pub fn new(subscription: ApprovalSubscription, snapshot: ApprovalView) -> Self {
        Self {
            subscription,
            snapshot,
        }
    }
}

/// One approval-state update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalUpdate {
    /// Initial state sent to a subscriber.
    Snapshot(ApprovalView),
    /// A new question became pending.
    QuestionReceived(ApprovalQuestion),
    /// The active question changed.
    QuestionSelected(ApprovalIdentifier),
    /// A question was answered and removed from the pending queue.
    QuestionAnswered(ApprovalResponse),
}

/// One update routed to one subscribed client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalDelivery {
    pub subscription: ApprovalSubscriptionIdentifier,
    pub client: ApprovalClientIdentifier,
    pub update: ApprovalUpdate,
}

impl ApprovalDelivery {
    pub fn new(
        subscription: ApprovalSubscriptionIdentifier,
        client: ApprovalClientIdentifier,
        update: ApprovalUpdate,
    ) -> Self {
        Self {
            subscription,
            client,
            update,
        }
    }
}

/// Result of recording a closed response to a pending question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalResponseOutcome {
    pub question: Option<ApprovalQuestion>,
    pub deliveries: Vec<ApprovalDelivery>,
}

impl ApprovalResponseOutcome {
    pub fn new(question: Option<ApprovalQuestion>, deliveries: Vec<ApprovalDelivery>) -> Self {
        Self {
            question,
            deliveries,
        }
    }
}

/// Approval state owned by the workbench model.
#[derive(Debug, Default, Clone)]
pub struct ApprovalState {
    pending: Vec<ApprovalQuestion>,
    selected: Option<ApprovalIdentifier>,
    answered: Vec<ApprovalResponse>,
    subscriptions: Vec<ApprovalSubscription>,
    next_subscription: u64,
}

impl ApprovalState {
    pub fn receive(&mut self, question: ApprovalQuestion) -> Vec<ApprovalDelivery> {
        let identifier = question.identifier;
        let delivered = question.clone();
        self.pending.push(question);
        if self.selected.is_none() {
            self.selected = Some(identifier);
        }
        self.deliveries_for(ApprovalUpdate::QuestionReceived(delivered))
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

    pub fn select_with_deliveries(
        &mut self,
        identifier: ApprovalIdentifier,
    ) -> Vec<ApprovalDelivery> {
        if self.select(identifier) {
            self.deliveries_for(ApprovalUpdate::QuestionSelected(identifier))
        } else {
            Vec::new()
        }
    }

    pub fn answer(&mut self, response: ApprovalResponse) -> Option<ApprovalQuestion> {
        self.answer_with_deliveries(response).question
    }

    pub fn answer_with_deliveries(
        &mut self,
        response: ApprovalResponse,
    ) -> ApprovalResponseOutcome {
        if response.decision == ApprovalDecision::Defer {
            self.select(response.identifier);
            return ApprovalResponseOutcome::new(None, Vec::new());
        }

        let Some(index) = self
            .pending
            .iter()
            .position(|question| question.identifier == response.identifier)
        else {
            return ApprovalResponseOutcome::new(None, Vec::new());
        };
        let question = self.pending.remove(index);
        self.answered.push(response.clone());
        self.selected = self
            .pending
            .get(index)
            .or_else(|| self.pending.last())
            .map(|next| next.identifier);
        let deliveries = self.deliveries_for(ApprovalUpdate::QuestionAnswered(response));
        ApprovalResponseOutcome::new(Some(question), deliveries)
    }

    pub fn subscribe(
        &mut self,
        client: ApprovalClientIdentifier,
        interest: ApprovalInterest,
    ) -> ApprovalSubscriptionReceipt {
        let identifier = ApprovalSubscriptionIdentifier::new(self.next_subscription);
        self.next_subscription += 1;
        let subscription = ApprovalSubscription::new(identifier, client, interest);
        self.subscriptions.push(subscription.clone());
        ApprovalSubscriptionReceipt::new(subscription, self.view())
    }

    pub fn unsubscribe(&mut self, identifier: ApprovalSubscriptionIdentifier) -> bool {
        let Some(index) = self
            .subscriptions
            .iter()
            .position(|subscription| subscription.identifier == identifier)
        else {
            return false;
        };
        self.subscriptions.remove(index);
        true
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

    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    pub fn view(&self) -> ApprovalView {
        ApprovalView {
            current: self.current().cloned(),
            pending_count: self.pending_count(),
            answered_count: self.answered_count(),
            subscription_count: self.subscription_count(),
        }
    }

    fn deliveries_for(&self, update: ApprovalUpdate) -> Vec<ApprovalDelivery> {
        self.subscriptions
            .iter()
            .filter_map(|subscription| subscription.delivery_for(&update))
            .collect()
    }
}

/// Pure-data approval snapshot for shells to paint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalView {
    pub current: Option<ApprovalQuestion>,
    pub pending_count: usize,
    pub answered_count: usize,
    pub subscription_count: usize,
}
