//! [`UserEvent`] and [`EngineEvent`] — the two event kinds the client model
//! accepts.
//!
//! Closed enums; one variant per gesture or daemon push. The model is
//! MVU-shaped: a shell forwards [`UserEvent`]s, the runtime raises
//! [`EngineEvent`]s, and both flow through
//! [`crate::observation::ObservationModel`] which returns [`crate::cmd::Cmd`]s.
//!
//! Every payload type here is a live `signal-mentci` /
//! `meta-signal-mentci` contract type — the model never mints a parallel
//! mirror vocabulary.

use meta_signal_mentci::ComponentSocketKind;
use signal_mentci::{
    AnswerProposal, ApprovalVerdict, InterfaceInterest, ProjectedInterfaceState, QuestionProposal,
    SubscriptionToken,
};

/// What a shell forwards when the psyche does something. Each variant maps to
/// a `signal-mentci` request the model turns into a [`crate::cmd::Cmd`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    /// Open a projected-state observation on a component socket with a chosen
    /// interest. The daemon mints the token; the model records it on the open
    /// reply.
    Observe {
        socket: ComponentSocketKind,
        interest: InterfaceInterest,
    },
    /// Close a previously-opened observation by its daemon-minted token.
    RetractObservation {
        socket: ComponentSocketKind,
        token: SubscriptionToken,
    },
    /// Select which pending question the approval surface is focused on.
    SelectQuestion {
        question: signal_mentci::QuestionIdentifier,
    },
    /// Submit a closed verdict on a pending question.
    AnswerQuestion { verdict: ApprovalVerdict },
    /// Author a different answer as a new typed proposal object on the normal
    /// authorization path (edits-as-proposals).
    ProposeEditedAnswer { proposal: AnswerProposal },
    /// Push a programmable interface mutation into the daemon's canonical
    /// state.
    PushQuestion {
        socket: ComponentSocketKind,
        proposal: QuestionProposal,
    },
}

/// What the runtime raises when something arrives from a daemon connection.
/// Replies and pushes are carried as the live `signal-mentci` reply / event
/// shapes; the model folds them into per-socket projected state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    /// A daemon accepted an observation and returned its minted token plus the
    /// opening projected snapshot.
    ObservationOpened {
        socket: ComponentSocketKind,
        opened: signal_mentci::InterfaceObservationOpened,
    },
    /// A subscription stream pushed a fresh projected state for a token the
    /// model holds.
    InterfaceStateChanged {
        socket: ComponentSocketKind,
        token: SubscriptionToken,
        state: ProjectedInterfaceState,
    },
    /// The daemon admitted a presented question and minted its identifier.
    QuestionPresented {
        socket: ComponentSocketKind,
        presented: signal_mentci::QuestionPresented,
    },
    /// The daemon accepted a verdict.
    VerdictAccepted {
        socket: ComponentSocketKind,
        accepted: signal_mentci::VerdictAccepted,
    },
    /// The daemon admitted an edited-answer proposal as a new object and
    /// returned the digest criome will later approve.
    AnswerProposalAdmitted {
        socket: ComponentSocketKind,
        admitted: signal_mentci::AnswerProposalAdmitted,
    },
    /// A daemon connection rejected a request.
    Rejected {
        socket: ComponentSocketKind,
        rejection: signal_mentci::Rejection,
    },
    /// A component socket connection changed liveness.
    ConnectionChanged {
        socket: ComponentSocketKind,
        liveness: SocketLiveness,
    },
}

/// Liveness of one component-socket connection, surfaced in the header view.
/// A typed enum, not a `bool` flag pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketLiveness {
    Disconnected,
    Connecting,
    Connected,
    Failed { reason: String },
}
