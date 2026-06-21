//! Behavioral tests for the re-founded shared model over the live contracts.

use meta_signal_criome::AuthorizationApprovalDecision;
use meta_signal_mentci::ComponentSocketKind;
use signal_criome::AuthorizationRequestSlot;
use signal_mentci::{
    AnswerText, ApprovalDecision, ApprovalQuestion, ApprovalSource, ApprovalVerdict,
    ExplanationText, InterfaceInterest, InterfaceObservationOpened, InterfaceProjection,
    InterfaceState, ProjectedInterfaceState, PromptText, QuestionIdentifier, QuestionProposal,
    RevisionCounter, StatusText, SubscriberName, SubscriptionToken,
};

use mentci_lib::approval::{ApprovalClientIdentifier, ApprovalInterest};
use mentci_lib::{Cmd, CriomeVerdict, EngineEvent, ObservationModel, SocketLiveness, UserEvent};

fn question(identifier: &str) -> ApprovalQuestion {
    question_with_source(identifier, ApprovalSource::AgentQuestion)
}

/// A criome-sourced question carrying the parked authorization slot — the
/// motivating case for the verdict-by-slot seam.
fn criome_question(identifier: &str, slot: &str) -> ApprovalQuestion {
    question_with_source(
        identifier,
        ApprovalSource::CriomeEscalation(AuthorizationRequestSlot::new(slot)),
    )
}

fn question_with_source(identifier: &str, source: ApprovalSource) -> ApprovalQuestion {
    ApprovalQuestion {
        identifier: QuestionIdentifier::new(identifier),
        proposal: QuestionProposal::new(
            source,
            PromptText::new("authorize the thing"),
            Some(AnswerText::new("approve")),
            ExplanationText::new("a question is pending"),
            Vec::new(),
        ),
    }
}

fn projected_with(questions: Vec<ApprovalQuestion>) -> ProjectedInterfaceState {
    ProjectedInterfaceState {
        revision: RevisionCounter::new(7),
        projection: InterfaceProjection::FullProjection(InterfaceState::new(
            RevisionCounter::new(7),
            StatusText::new("ready"),
            None,
            Vec::new(),
            questions,
        )),
    }
}

#[test]
fn observe_emits_a_request_and_marks_connecting() {
    let mut model = ObservationModel::new(SubscriberName::new("test-client"));
    let commands = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    assert_eq!(commands.len(), 1);
    let Cmd::SendRequest { socket, request } = &commands[0];
    assert_eq!(*socket, ComponentSocketKind::Mentci);
    assert!(matches!(
        request,
        signal_mentci::Input::ObserveInterfaceState(_)
    ));
    let slot = model.socket(ComponentSocketKind::Mentci).unwrap();
    assert!(matches!(slot.liveness(), SocketLiveness::Connecting));
}

#[test]
fn opened_observation_folds_pending_into_the_approval_cursor() {
    let mut model = ObservationModel::new(SubscriberName::new("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            token: SubscriptionToken::new("subscription-1"),
            state: projected_with(vec![question("question-1"), question("question-2")]),
        },
    });

    let slot = model.socket(ComponentSocketKind::Mentci).unwrap();
    assert!(matches!(slot.liveness(), SocketLiveness::Connected));
    assert_eq!(slot.token().unwrap().as_str(), "subscription-1");

    let approval = model.approval();
    assert_eq!(approval.pending().len(), 2);
    // First pending question is auto-selected.
    assert_eq!(
        approval.current().unwrap().identifier.as_str(),
        "question-1"
    );
}

#[test]
fn pushed_state_change_refolds_the_queue() {
    let mut model = ObservationModel::new(SubscriberName::new("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            token: SubscriptionToken::new("subscription-1"),
            state: projected_with(vec![question("question-1")]),
        },
    });
    model.on_engine_event(EngineEvent::InterfaceStateChanged {
        socket: ComponentSocketKind::Mentci,
        token: SubscriptionToken::new("subscription-1"),
        state: projected_with(vec![question("question-1"), question("question-2")]),
    });
    assert_eq!(model.approval().pending().len(), 2);

    // A push on a token we don't hold is ignored.
    model.on_engine_event(EngineEvent::InterfaceStateChanged {
        socket: ComponentSocketKind::Mentci,
        token: SubscriptionToken::new("not-our-token"),
        state: projected_with(Vec::new()),
    });
    assert_eq!(model.approval().pending().len(), 2);
}

#[test]
fn answering_a_question_emits_a_verdict_and_drops_it_from_pending() {
    let mut model = ObservationModel::new(SubscriberName::new("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            token: SubscriptionToken::new("subscription-1"),
            state: projected_with(vec![question("question-1"), question("question-2")]),
        },
    });

    let verdict = ApprovalVerdict {
        question: QuestionIdentifier::new("question-1"),
        decision: ApprovalDecision::ApproveSuggestedAnswer,
        answered_by: SubscriberName::new("test-client"),
    };
    let commands = model.on_user_event(UserEvent::AnswerQuestion { verdict });
    assert_eq!(commands.len(), 1);
    assert!(matches!(
        &commands[0],
        Cmd::SendRequest {
            request: signal_mentci::Input::AnswerQuestion(_),
            ..
        }
    ));
    assert_eq!(model.approval().pending().len(), 1);
    assert_eq!(model.approval().answered().len(), 1);
}

#[test]
fn answering_a_criome_question_sends_the_verdict_to_the_daemon() {
    // Daemon-routing: the client never opens a criome socket; it sends
    // AnswerQuestion to the mentci daemon, which routes to criome by the slot.
    let mut model = ObservationModel::new(SubscriberName::new("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            token: SubscriptionToken::new("subscription-1"),
            state: projected_with(vec![criome_question("question-1", "slot-1")]),
        },
    });
    let commands = model.on_user_event(UserEvent::AnswerQuestion {
        verdict: ApprovalVerdict {
            question: QuestionIdentifier::new("question-1"),
            decision: ApprovalDecision::ApproveSuggestedAnswer,
            answered_by: SubscriberName::new("test-client"),
        },
    });
    assert_eq!(commands.len(), 1);
    assert!(matches!(
        &commands[0],
        Cmd::SendRequest {
            socket: ComponentSocketKind::Mentci,
            request: signal_mentci::Input::AnswerQuestion(_),
        }
    ));
    // The criome-sourced question leaves the pending queue once closed.
    assert_eq!(model.approval().pending().len(), 0);
    assert_eq!(model.approval().answered().len(), 1);
}

#[test]
fn defer_keeps_the_question_pending() {
    let mut approval = mentci_lib::approval::ApprovalModel::default();
    let _ = approval.absorb_pending(vec![question("question-1")]);
    let outcome = approval.answer(ApprovalVerdict {
        question: QuestionIdentifier::new("question-1"),
        decision: ApprovalDecision::Defer,
        answered_by: SubscriberName::new("test-client"),
    });
    assert!(outcome.answered().is_none());
    assert_eq!(approval.pending().len(), 1);
}

#[test]
fn subscriptions_receive_deliveries_on_state_change() {
    let mut approval = mentci_lib::approval::ApprovalModel::default();
    let receipt = approval.subscribe(
        ApprovalClientIdentifier::new(1),
        ApprovalInterest::PendingQuestions,
    );
    assert_eq!(receipt.snapshot().pending_count, 0);
    let deliveries = approval.absorb_pending(vec![question("question-1")]);
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].client(), ApprovalClientIdentifier::new(1));
}

#[test]
fn closed_decision_maps_to_the_criome_verdict() {
    let slot = AuthorizationRequestSlot::new("request-slot-42");
    let approve =
        CriomeVerdict::from_decision(slot.clone(), ApprovalDecision::ApproveSuggestedAnswer);
    assert_eq!(approve.decision(), AuthorizationApprovalDecision::Approve);
    assert_eq!(approve.request_slot().payload(), slot.payload());

    let reject = CriomeVerdict::from_decision(slot.clone(), ApprovalDecision::Reject);
    assert_eq!(reject.decision(), AuthorizationApprovalDecision::Reject);

    let defer = CriomeVerdict::from_decision(slot, ApprovalDecision::Defer);
    assert_eq!(defer.decision(), AuthorizationApprovalDecision::Defer);
}

#[cfg(feature = "nota-text")]
#[test]
fn nota_fallback_renders_a_typed_reply() {
    use mentci_lib::{RenderNota, RenderOrigin};
    let presented = signal_mentci::QuestionPresented {
        question: QuestionIdentifier::new("question-1"),
        revision: RevisionCounter::new(1),
        accepted_at: signal_mentci::TimestampNanos::new(0),
    };
    let rendered = presented.render_nota(RenderOrigin::Reply);
    assert_eq!(rendered.origin().label(), "reply");
    assert!(rendered.body().contains("question-1"));
}

#[test]
fn view_carries_one_row_per_observed_socket() {
    let mut model = ObservationModel::new(SubscriberName::new("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Criome,
        interest: InterfaceInterest::PendingQuestions,
    });
    let view = model.view();
    assert_eq!(view.sockets.len(), 2);
}
