//! The approval state machine — KEPT from the original design, REBASED onto
//! `signal-mentci`'s approval vocabulary.
//!
//! The original `mentci-lib` hand-rolled its own `ApprovalQuestion`,
//! `ApprovalSource`, `ApprovalDecision`, `ApprovalResponse`,
//! `AnswerProposal`, prompt/answer/explanation/context newtypes. `signal-mentci`
//! now OWNS every one of those (it is the working wire contract). So this module
//! keeps the *logic* — the pending queue, the selection cursor, the
//! subscription/delivery fan-out, the closed-verdict + edits-as-proposals
//! shapes — and consumes the contract types instead of redefining them.
//!
//! What the daemon owns (minting question identifiers, the canonical
//! `InterfaceState`) is NOT duplicated here: the model reads the daemon's
//! projected pending-question list. This state machine is the *client-side*
//! cursor + the local subscription fan-out a control surface needs.

use signal_mentci::{
    AnswerProposal, ApprovalDecision, ApprovalQuestion, ApprovalVerdict, QuestionIdentifier,
    SubscriberName,
};

/// Local identifier for a client subscribed to approval-state changes the
/// model fans out (a status bar, a popup, an approval pane). Daemon
/// subscription tokens are a different identity, minted by the daemon.
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

/// Local identifier for one approval-state subscription the model holds.
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

/// Which approval-state updates a local client wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalInterest {
    All,
    PendingQuestions,
    AnsweredVerdicts,
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
            Self::AnsweredVerdicts => {
                matches!(
                    update,
                    ApprovalUpdate::Snapshot(_) | ApprovalUpdate::QuestionAnswered(_)
                )
            }
        }
    }
}

/// One local client subscription over the approval state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalSubscription {
    identifier: ApprovalSubscriptionIdentifier,
    client: ApprovalClientIdentifier,
    interest: ApprovalInterest,
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

    pub fn identifier(&self) -> ApprovalSubscriptionIdentifier {
        self.identifier
    }

    pub fn client(&self) -> ApprovalClientIdentifier {
        self.client
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

/// One approval-state update the model fans out to local subscribers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalUpdate {
    Snapshot(ApprovalView),
    QuestionReceived(ApprovalQuestion),
    QuestionSelected(QuestionIdentifier),
    QuestionAnswered(ApprovalVerdict),
}

/// One update routed to one local subscriber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalDelivery {
    subscription: ApprovalSubscriptionIdentifier,
    client: ApprovalClientIdentifier,
    update: ApprovalUpdate,
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

    pub fn subscription(&self) -> ApprovalSubscriptionIdentifier {
        self.subscription
    }

    pub fn client(&self) -> ApprovalClientIdentifier {
        self.client
    }

    pub fn update(&self) -> &ApprovalUpdate {
        &self.update
    }
}

/// Receipt returned when a local client subscribes: the subscription plus the
/// snapshot it opens with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalSubscriptionReceipt {
    subscription: ApprovalSubscription,
    snapshot: ApprovalView,
}

impl ApprovalSubscriptionReceipt {
    pub fn subscription(&self) -> &ApprovalSubscription {
        &self.subscription
    }

    pub fn snapshot(&self) -> &ApprovalView {
        &self.snapshot
    }
}

/// The result of answering: the question removed from the queue (if any) plus
/// the deliveries the verdict generated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalAnswerOutcome {
    answered: Option<ApprovalQuestion>,
    verdict: Option<ApprovalVerdict>,
    deliveries: Vec<ApprovalDelivery>,
}

impl ApprovalAnswerOutcome {
    pub fn answered(&self) -> Option<&ApprovalQuestion> {
        self.answered.as_ref()
    }

    pub fn verdict(&self) -> Option<&ApprovalVerdict> {
        self.verdict.as_ref()
    }

    pub fn deliveries(&self) -> &[ApprovalDelivery] {
        &self.deliveries
    }
}

/// The client-side approval state: the pending-question queue (mirrored from
/// the daemon's projected state), the selection cursor, the answered-verdict
/// log, and local subscriptions. This is the rebased version of the original
/// `ApprovalState`.
#[derive(Debug, Default, Clone)]
pub struct ApprovalModel {
    pending: Vec<ApprovalQuestion>,
    selected: Option<QuestionIdentifier>,
    answered: Vec<ApprovalVerdict>,
    subscriptions: Vec<ApprovalSubscription>,
    next_subscription: u64,
}

impl ApprovalModel {
    /// Replace the pending queue with the daemon's projected list, preserving
    /// the selection if it survives and fanning out a snapshot. This is how a
    /// fresh `PendingQuestionsProjection` or `FullProjection` folds in.
    pub fn absorb_pending(&mut self, pending: Vec<ApprovalQuestion>) -> Vec<ApprovalDelivery> {
        self.pending = pending;
        let selection_survives = self
            .selected
            .as_ref()
            .is_some_and(|identifier| self.is_pending(identifier));
        if !selection_survives {
            self.selected = self
                .pending
                .first()
                .map(|question| question.identifier.clone());
        }
        self.deliveries_for(ApprovalUpdate::Snapshot(self.view()))
    }

    /// Register a newly-arrived question locally (used when a single question
    /// push arrives rather than a whole projection).
    pub fn receive(&mut self, question: ApprovalQuestion) -> Vec<ApprovalDelivery> {
        let identifier = question.identifier.clone();
        let received = question.clone();
        self.pending.push(question);
        if self.selected.is_none() {
            self.selected = Some(identifier);
        }
        self.deliveries_for(ApprovalUpdate::QuestionReceived(received))
    }

    /// Move the selection cursor to a pending question.
    pub fn select(&mut self, identifier: QuestionIdentifier) -> Vec<ApprovalDelivery> {
        if self.is_pending(&identifier) {
            self.selected = Some(identifier.clone());
            self.deliveries_for(ApprovalUpdate::QuestionSelected(identifier))
        } else {
            Vec::new()
        }
    }

    /// Record a closed verdict against a pending question. `Defer` leaves the
    /// question pending and only moves the cursor; a binding verdict removes
    /// the question and logs the verdict, advancing the cursor.
    pub fn answer(&mut self, verdict: ApprovalVerdict) -> ApprovalAnswerOutcome {
        if matches!(verdict.decision, ApprovalDecision::Defer) {
            let _ = self.select(verdict.question.clone());
            return ApprovalAnswerOutcome {
                answered: None,
                verdict: Some(verdict),
                deliveries: Vec::new(),
            };
        }
        let Some(index) = self
            .pending
            .iter()
            .position(|question| question.identifier == verdict.question)
        else {
            return ApprovalAnswerOutcome {
                answered: None,
                verdict: None,
                deliveries: Vec::new(),
            };
        };
        let answered = self.pending.remove(index);
        self.answered.push(verdict.clone());
        self.selected = self
            .pending
            .get(index)
            .or_else(|| self.pending.last())
            .map(|next| next.identifier.clone());
        let deliveries = self.deliveries_for(ApprovalUpdate::QuestionAnswered(verdict.clone()));
        ApprovalAnswerOutcome {
            answered: Some(answered),
            verdict: Some(verdict),
            deliveries,
        }
    }

    /// Build a closed verdict for the currently-selected question (the common
    /// "approve/reject/defer the focused question" gesture). Returns `None`
    /// when nothing is selected.
    pub fn verdict_for_selected(
        &self,
        decision: ApprovalDecision,
        answered_by: SubscriberName,
    ) -> Option<ApprovalVerdict> {
        let question = self.selected.clone()?;
        Some(ApprovalVerdict {
            question,
            decision,
            answered_by,
        })
    }

    /// Build an edited-answer proposal against the selected question
    /// (edits-as-proposals: the body re-enters the normal authorization path
    /// as a new object, NOT a verdict variant).
    pub fn proposal_for_selected(
        &self,
        body: signal_mentci::AnswerText,
        authored_by: SubscriberName,
    ) -> Option<AnswerProposal> {
        let question = self.selected.clone()?;
        Some(AnswerProposal {
            question,
            body,
            authored_by,
        })
    }

    /// Subscribe a local client; returns the receipt carrying the opening
    /// snapshot.
    pub fn subscribe(
        &mut self,
        client: ApprovalClientIdentifier,
        interest: ApprovalInterest,
    ) -> ApprovalSubscriptionReceipt {
        let identifier = ApprovalSubscriptionIdentifier::new(self.next_subscription);
        self.next_subscription += 1;
        let subscription = ApprovalSubscription::new(identifier, client, interest);
        self.subscriptions.push(subscription.clone());
        ApprovalSubscriptionReceipt {
            subscription,
            snapshot: self.view(),
        }
    }

    /// Remove a local subscription. Returns whether it existed.
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
        let selected = self.selected.as_ref()?;
        self.pending
            .iter()
            .find(|question| &question.identifier == selected)
    }

    pub fn pending(&self) -> &[ApprovalQuestion] {
        &self.pending
    }

    pub fn answered(&self) -> &[ApprovalVerdict] {
        &self.answered
    }

    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    pub fn view(&self) -> ApprovalView {
        ApprovalView {
            current: self.current().cloned(),
            pending_count: self.pending.len(),
            answered_count: self.answered.len(),
            subscription_count: self.subscriptions.len(),
        }
    }

    fn is_pending(&self, identifier: &QuestionIdentifier) -> bool {
        self.pending
            .iter()
            .any(|question| &question.identifier == identifier)
    }

    fn deliveries_for(&self, update: ApprovalUpdate) -> Vec<ApprovalDelivery> {
        self.subscriptions
            .iter()
            .filter_map(|subscription| subscription.delivery_for(&update))
            .collect()
    }
}

/// Pure-data approval snapshot a shell paints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalView {
    pub current: Option<ApprovalQuestion>,
    pub pending_count: usize,
    pub answered_count: usize,
    pub subscription_count: usize,
}
