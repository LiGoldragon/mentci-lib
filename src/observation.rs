//! [`ObservationModel`] — the client observability + control core.
//!
//! This is the re-founded heart of mentci-lib: one MVU-shaped model thin
//! clients share. It is keyed by component socket
//! ([`ComponentSocketKind`] from `meta-signal-mentci` / `signal-standard`):
//! mentci can observe its own canonical state, the meta surface, and a criome
//! peer, each on its own connection, each folding into its own
//! [`SocketObservation`] slot.
//!
//! The contract is `update(state, event) -> (state, Vec<Cmd>)` and
//! `view(state) -> ObservationView`, exactly the original MVU shape — but every
//! payload is a live `signal-mentci` type and the side-effects carry
//! `MentciRequest`s, not the dead graph-signal frames.

use std::collections::BTreeMap;

use meta_signal_mentci::ComponentSocketKind;
use signal_mentci::{
    CriomeAccess, InterfaceInterest, InterfaceProjection, InterfaceStateObservation, MentciRequest,
    PaneContent, ProjectedInterfaceState, QuestionProposal, RevisionCounter, SubscriberName,
    SubscriptionToken,
};

use crate::approval::{ApprovalModel, ApprovalView};
use crate::cmd::Cmd;
use crate::error::ComponentSocketLabel;
use crate::event::{EngineEvent, SocketLiveness, UserEvent};

/// One component socket's observation state. Holds the interest the model
/// subscribed with, the daemon-minted token once the observation is open, the
/// latest projected state folded in, and the connection liveness.
#[derive(Debug, Clone)]
pub struct SocketObservation {
    interest: Option<InterfaceInterest>,
    token: Option<SubscriptionToken>,
    latest: Option<ProjectedInterfaceState>,
    liveness: SocketLiveness,
}

impl Default for SocketObservation {
    fn default() -> Self {
        Self {
            interest: None,
            token: None,
            latest: None,
            liveness: SocketLiveness::Disconnected,
        }
    }
}

impl SocketObservation {
    pub fn token(&self) -> Option<&SubscriptionToken> {
        self.token.as_ref()
    }

    pub fn interest(&self) -> Option<InterfaceInterest> {
        self.interest
    }

    pub fn latest(&self) -> Option<&ProjectedInterfaceState> {
        self.latest.as_ref()
    }

    pub fn liveness(&self) -> &SocketLiveness {
        &self.liveness
    }

    /// The revision counter of the latest folded state, if any.
    pub fn revision(&self) -> Option<RevisionCounter> {
        self.latest
            .as_ref()
            .map(|state| state.revision_counter.clone())
    }
}

/// The client model. One per mentci session. Owns per-socket observations and
/// the client-side approval cursor. The subscriber name identities this model
/// to every daemon it observes.
#[derive(Debug, Clone)]
pub struct ObservationModel {
    subscriber: SubscriberName,
    sockets: BTreeMap<ComponentSocketKey, SocketObservation>,
    approval: ApprovalModel,
}

/// A `ComponentSocketKind` is not `Ord`, so it cannot key a `BTreeMap`
/// directly. This newtype gives it a stable total order keyed by the typed
/// label — a named contact point rather than scattering the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ComponentSocketKey(u8);

impl From<ComponentSocketKind> for ComponentSocketKey {
    fn from(kind: ComponentSocketKind) -> Self {
        Self(match kind {
            ComponentSocketKind::Mentci => 0,
            ComponentSocketKind::MetaMentci => 1,
            ComponentSocketKind::Criome => 2,
            ComponentSocketKind::MetaCriome => 3,
            ComponentSocketKind::Introspect => 4,
            ComponentSocketKind::MetaIntrospect => 5,
        })
    }
}

impl ComponentSocketKey {
    fn kind(self) -> ComponentSocketKind {
        match self.0 {
            0 => ComponentSocketKind::Mentci,
            1 => ComponentSocketKind::MetaMentci,
            2 => ComponentSocketKind::Criome,
            3 => ComponentSocketKind::MetaCriome,
            4 => ComponentSocketKind::Introspect,
            _ => ComponentSocketKind::MetaIntrospect,
        }
    }
}

impl ObservationModel {
    /// Construct a fresh model identified by a subscriber name. No
    /// observations are open until the runtime dispatches the
    /// [`Cmd::SendRequest`]s the model returns.
    pub fn new(subscriber: SubscriberName) -> Self {
        Self {
            subscriber,
            sockets: BTreeMap::new(),
            approval: ApprovalModel::default(),
        }
    }

    /// Read the approval cursor (selection, queue, fan-out state).
    pub fn approval(&self) -> &ApprovalModel {
        &self.approval
    }

    /// The subscriber name this model presents to every daemon it observes.
    pub fn subscriber(&self) -> &SubscriberName {
        &self.subscriber
    }

    /// Read one socket's observation slot.
    pub fn socket(&self, kind: ComponentSocketKind) -> Option<&SocketObservation> {
        self.sockets.get(&ComponentSocketKey::from(kind))
    }

    /// Apply a user gesture. Returns the side-effects the runtime dispatches.
    pub fn on_user_event(&mut self, event: UserEvent) -> Vec<Cmd> {
        match event {
            UserEvent::Observe { socket, interest } => {
                let slot = self.slot_mut(socket);
                slot.interest = Some(interest);
                slot.liveness = SocketLiveness::Connecting;
                vec![Cmd::send(
                    socket,
                    MentciRequest::ObserveInterfaceState(InterfaceStateObservation {
                        subscriber_name: self.subscriber.clone(),
                        interface_interest: interest,
                    }),
                )]
            }
            UserEvent::RetractObservation { socket, token } => {
                vec![Cmd::send(
                    socket,
                    MentciRequest::RetractInterfaceObservation(token),
                )]
            }
            UserEvent::SelectQuestion { question } => {
                // Local cursor move; the daemon owns no per-client selection,
                // so no Cmd leaves the model.
                let _ = self.approval.select(question);
                Vec::new()
            }
            UserEvent::AnswerQuestion { verdict } => {
                // Clients reach criome only through the mentci daemon: a closed
                // answer (criome-sourced or not) goes to the daemon over the
                // mentci socket, and the daemon routes criome-sourced verdicts
                // to criome by the parked slot (daemon-routing decision). The
                // client never opens a criome socket; the slot the source
                // carries is what lets the daemon route.
                let outcome = self.approval.answer(verdict.clone());
                if outcome.verdict().is_some() {
                    vec![Cmd::send(
                        ComponentSocketKind::Mentci,
                        MentciRequest::AnswerQuestion(verdict),
                    )]
                } else {
                    Vec::new()
                }
            }
            UserEvent::ProposeEditedAnswer { proposal } => {
                vec![Cmd::send(
                    ComponentSocketKind::Mentci,
                    MentciRequest::ProposeEditedAnswer(proposal),
                )]
            }
            UserEvent::PushQuestion { socket, proposal } => {
                vec![Cmd::send(socket, MentciRequest::PresentQuestion(proposal))]
            }
        }
    }

    /// Fold a daemon-originated event into the model. Returns any follow-up
    /// side-effects (none today; the model is push-driven).
    pub fn on_engine_event(&mut self, event: EngineEvent) -> Vec<Cmd> {
        match event {
            EngineEvent::ObservationOpened { socket, opened } => {
                let slot = self.slot_mut(socket);
                slot.token = Some(opened.subscription_token);
                slot.liveness = SocketLiveness::Connected;
                self.fold_projection(socket, opened.projected_interface_state);
                Vec::new()
            }
            EngineEvent::InterfaceStateChanged {
                socket,
                token,
                state,
            } => {
                if self
                    .socket(socket)
                    .and_then(|slot| slot.token.clone())
                    .as_ref()
                    != Some(&token)
                {
                    // A push for a token we don't hold — ignore rather than
                    // fold foreign state.
                    return Vec::new();
                }
                self.fold_projection(socket, state);
                Vec::new()
            }
            EngineEvent::QuestionPresented { .. }
            | EngineEvent::VerdictAccepted { .. }
            | EngineEvent::AnswerProposalAdmitted { .. }
            | EngineEvent::Rejected { .. } => {
                // Acknowledgements: the canonical change arrives as the next
                // InterfaceStateChanged push, which re-folds the queue.
                Vec::new()
            }
            EngineEvent::ConnectionChanged { socket, liveness } => {
                self.slot_mut(socket).liveness = liveness;
                Vec::new()
            }
        }
    }

    /// Derive the per-frame snapshot. Pure; takes `&self`.
    pub fn view(&self) -> ObservationView {
        ObservationView {
            sockets: self
                .sockets
                .iter()
                .map(|(key, slot)| SocketView {
                    socket: ComponentSocketLabel::from(key.kind()),
                    liveness: slot.liveness.clone(),
                    revision: slot.revision(),
                    has_state: slot.latest.is_some(),
                })
                .collect(),
            approval: self.approval.view(),
            panes: self
                .socket(ComponentSocketKind::Mentci)
                .and_then(|slot| slot.latest())
                .map(Self::panes_from_projection)
                .unwrap_or_default(),
            criome_access: self
                .socket(ComponentSocketKind::Mentci)
                .and_then(|slot| slot.latest())
                .and_then(ProjectedInterfaceState::criome_access),
        }
    }

    /// Fold one projected state into a socket slot and refresh the approval
    /// cursor from its pending-question slice.
    fn fold_projection(&mut self, socket: ComponentSocketKind, state: ProjectedInterfaceState) {
        let pending = state.pending_questions().to_vec();
        self.slot_mut(socket).latest = Some(state);
        let _ = self.approval.absorb_pending(pending);
    }

    fn slot_mut(&mut self, kind: ComponentSocketKind) -> &mut SocketObservation {
        self.sockets
            .entry(ComponentSocketKey::from(kind))
            .or_default()
    }

    fn panes_from_projection(state: &ProjectedInterfaceState) -> Vec<PaneContent> {
        match &state.interface_projection {
            InterfaceProjection::FullProjection(state) => state.panes().to_vec(),
            InterfaceProjection::StatusProjection(_)
            | InterfaceProjection::NotificationProjection(_)
            | InterfaceProjection::PendingQuestionsProjection(_) => Vec::new(),
        }
    }
}

/// The per-frame snapshot a shell paints. Pure data, no GUI types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationView {
    /// One header row per observed component socket.
    pub sockets: Vec<SocketView>,
    /// The approval surface snapshot.
    pub approval: ApprovalView,
    /// Open daemon-projected panes from the latest full Mentci observation.
    pub panes: Vec<PaneContent>,
    /// The criome access level mirrored from the mentci daemon's latest full
    /// projection. `None` until a full projection is folded; clients treat
    /// `None` as observation-only.
    pub criome_access: Option<CriomeAccess>,
}

/// One observed socket's header row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketView {
    pub socket: ComponentSocketLabel,
    pub liveness: SocketLiveness,
    pub revision: Option<RevisionCounter>,
    pub has_state: bool,
}

/// Form the gesture that presents a question on a socket — a method on the
/// proposal noun, not a free constructor floating beside it.
pub trait PresentableQuestion {
    fn present_on(self, socket: ComponentSocketKind) -> UserEvent;
}

impl PresentableQuestion for QuestionProposal {
    fn present_on(self, socket: ComponentSocketKind) -> UserEvent {
        UserEvent::PushQuestion {
            socket,
            proposal: self,
        }
    }
}
