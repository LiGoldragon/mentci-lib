//! Behavioral tests for the re-founded shared model over the live contracts.

use meta_signal_criome::AuthorizationApprovalDecision;
use meta_signal_mentci::ComponentSocketKind;
use signal_criome::AuthorizationRequestSlot;
use signal_mentci::{
    AnswerText, ApprovalDecision, ApprovalQuestion, ApprovalSource, ApprovalVerdict, ContextBody,
    ContextLabel, ExplanationText, InterfaceInterest, InterfaceObservationOpened,
    InterfaceProjection, InterfaceState, NotificationText, PaneContent, PaneLabel,
    ProjectedInterfaceState, PromptText, QuestionContext, QuestionIdentifier, QuestionProposal,
    StatusText, SubscriberName, SubscriptionToken,
};

use mentci_lib::approval::{ApprovalClientIdentifier, ApprovalInterest, ApprovalUpdate};
use mentci_lib::{
    Cmd, CriomeAccess, CriomeVerdict, EngineEvent, ObservationModel, SocketLiveness, UserEvent,
};

fn question(identifier: &str) -> ApprovalQuestion {
    question_with_source(identifier, ApprovalSource::AgentQuestion)
}

/// A criome-sourced question carrying the parked authorization slot — the
/// motivating case for the verdict-by-slot seam.
fn criome_question(identifier: &str, slot: &str) -> ApprovalQuestion {
    question_with_source(
        identifier,
        ApprovalSource::CriomeEscalation(AuthorizationRequestSlot::from(slot)),
    )
}

fn question_with_source(identifier: &str, source: ApprovalSource) -> ApprovalQuestion {
    ApprovalQuestion {
        question_identifier: QuestionIdentifier::from(identifier),
        question_proposal: QuestionProposal {
            approval_source: source,
            prompt_text: PromptText::from("authorize the thing"),
            answer_text: AnswerText::from("approve"),
            explanation_text: ExplanationText::from("a question is pending"),
            question_context: QuestionContext {
                context_label: ContextLabel::from("subject"),
                context_body: ContextBody::from("the thing"),
            },
        },
    }
}

fn pane(label: &str, body: &str) -> PaneContent {
    PaneContent {
        pane_label: PaneLabel::from(label),
        context_body: ContextBody::from(body),
    }
}

/// A full projection carrying one approval question. The contract projects one
/// question at a time, so a helper that took a queue would be lying about the
/// wire.
fn projected_with(question: ApprovalQuestion) -> ProjectedInterfaceState {
    projected_with_access(question, CriomeAccess::ReadWrite)
}

fn projected_with_access(
    question: ApprovalQuestion,
    criome_access: CriomeAccess,
) -> ProjectedInterfaceState {
    projected_with_pane(question, pane("main", "waiting"), criome_access)
}

fn projected_with_pane(
    question: ApprovalQuestion,
    pane_content: PaneContent,
    criome_access: CriomeAccess,
) -> ProjectedInterfaceState {
    ProjectedInterfaceState {
        revision_counter: 7,
        interface_projection: InterfaceProjection::FullProjection(InterfaceState {
            revision_counter: 7,
            status_text: StatusText::from("ready"),
            notification_text: NotificationText::from("one question waiting"),
            pane_content,
            approval_question: question,
            criome_access,
        }),
    }
}

#[test]
fn observe_emits_a_request_and_marks_connecting() {
    let mut model = ObservationModel::new(SubscriberName::from("test-client"));
    let commands = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    assert_eq!(commands.len(), 1);
    let Cmd::SendRequest { socket, request } = &commands[0];
    assert_eq!(*socket, ComponentSocketKind::Mentci);
    assert!(matches!(
        request,
        signal_mentci::Query::ObserveInterfaceState(_)
    ));
    let slot = model.socket(ComponentSocketKind::Mentci).unwrap();
    assert!(matches!(slot.liveness(), SocketLiveness::Connecting));
}

#[test]
fn opened_observation_folds_pending_into_the_approval_cursor() {
    let mut model = ObservationModel::new(SubscriberName::from("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            subscription_token: SubscriptionToken::from("subscription-1"),
            projected_interface_state: projected_with(question("question-1")),
        },
    });

    let slot = model.socket(ComponentSocketKind::Mentci).unwrap();
    assert!(matches!(slot.liveness(), SocketLiveness::Connected));
    assert_eq!(slot.token().unwrap().as_str(), "subscription-1");

    let approval = model.approval();
    // The contract projects one approval question at a time, so a fold
    // contributes exactly one — and it is auto-selected.
    assert_eq!(approval.pending().len(), 1);
    assert_eq!(
        approval.current().unwrap().question_identifier,
        "question-1"
    );
}

#[test]
fn a_pushed_state_change_replaces_the_projected_question() {
    let mut model = ObservationModel::new(SubscriberName::from("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            subscription_token: SubscriptionToken::from("subscription-1"),
            projected_interface_state: projected_with(question("question-1")),
        },
    });
    model.on_engine_event(EngineEvent::InterfaceStateChanged {
        socket: ComponentSocketKind::Mentci,
        token: SubscriptionToken::from("subscription-1"),
        state: projected_with(question("question-2")),
    });
    // A refold replaces the projected question rather than accumulating: the
    // projection is the daemon's current state, not a log.
    assert_eq!(
        model.approval().current().unwrap().question_identifier,
        "question-2"
    );

    // A push on a token we don't hold is ignored.
    model.on_engine_event(EngineEvent::InterfaceStateChanged {
        socket: ComponentSocketKind::Mentci,
        token: SubscriptionToken::from("not-our-token"),
        state: projected_with(question("question-3")),
    });
    assert_eq!(
        model.approval().current().unwrap().question_identifier,
        "question-2"
    );
}

#[test]
fn answering_a_question_emits_a_verdict_and_drops_it_from_pending() {
    let mut model = ObservationModel::new(SubscriberName::from("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            subscription_token: SubscriptionToken::from("subscription-1"),
            projected_interface_state: projected_with(question("question-1")),
        },
    });

    let verdict = ApprovalVerdict {
        question_identifier: QuestionIdentifier::from("question-1"),
        approval_decision: ApprovalDecision::ApproveSuggestedAnswer,
        subscriber_name: SubscriberName::from("test-client"),
    };
    let commands = model.on_user_event(UserEvent::AnswerQuestion { verdict });
    assert_eq!(commands.len(), 1);
    assert!(matches!(
        &commands[0],
        Cmd::SendRequest {
            request: signal_mentci::Query::AnswerQuestion(_),
            ..
        }
    ));
    assert_eq!(model.approval().pending().len(), 0);
    assert_eq!(model.approval().answered().len(), 1);
}

#[test]
fn answering_a_criome_question_sends_the_verdict_to_the_daemon() {
    // Daemon-routing: the client never opens a criome socket; it sends
    // AnswerQuestion to the mentci daemon, which routes to criome by the slot.
    let mut model = ObservationModel::new(SubscriberName::from("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });
    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            subscription_token: SubscriptionToken::from("subscription-1"),
            projected_interface_state: projected_with(criome_question("question-1", "slot-1")),
        },
    });
    let commands = model.on_user_event(UserEvent::AnswerQuestion {
        verdict: ApprovalVerdict {
            question_identifier: QuestionIdentifier::from("question-1"),
            approval_decision: ApprovalDecision::ApproveSuggestedAnswer,
            subscriber_name: SubscriberName::from("test-client"),
        },
    });
    assert_eq!(commands.len(), 1);
    assert!(matches!(
        &commands[0],
        Cmd::SendRequest {
            socket: ComponentSocketKind::Mentci,
            request: signal_mentci::Query::AnswerQuestion(_),
        }
    ));
    // The criome-sourced question leaves the pending queue once closed.
    assert_eq!(model.approval().pending().len(), 0);
    assert_eq!(model.approval().answered().len(), 1);
}

#[test]
fn defer_keeps_the_question_pending() {
    let mut approval = mentci_lib::approval::ApprovalModel::default();
    let _receipt = approval.subscribe(
        ApprovalClientIdentifier::new(1),
        ApprovalInterest::PendingQuestions,
    );
    let _ = approval.absorb_pending(vec![question("question-1")]);
    let outcome = approval.answer(ApprovalVerdict {
        question_identifier: QuestionIdentifier::from("question-1"),
        approval_decision: ApprovalDecision::Defer,
        subscriber_name: SubscriberName::from("test-client"),
    });
    assert!(outcome.answered().is_none());
    assert_eq!(approval.pending().len(), 1);
    assert_eq!(outcome.deliveries().len(), 1);
    assert_eq!(
        outcome.deliveries()[0].client(),
        ApprovalClientIdentifier::new(1)
    );
    assert!(matches!(
        outcome.deliveries()[0].update(),
        ApprovalUpdate::QuestionSelected(question) if question == &QuestionIdentifier::from("question-1")
    ));
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
    let slot = AuthorizationRequestSlot::from("request-slot-42");
    let approve =
        CriomeVerdict::from_decision(slot.clone(), ApprovalDecision::ApproveSuggestedAnswer);
    assert_eq!(approve.decision(), &AuthorizationApprovalDecision::Approve);
    assert_eq!(approve.request_slot(), &slot);

    let reject = CriomeVerdict::from_decision(slot.clone(), ApprovalDecision::Reject);
    assert_eq!(reject.decision(), &AuthorizationApprovalDecision::Reject);

    let defer = CriomeVerdict::from_decision(slot, ApprovalDecision::Defer);
    assert_eq!(defer.decision(), &AuthorizationApprovalDecision::Defer);
}

#[cfg(feature = "datom")]
#[test]
fn datom_fallback_renders_a_typed_reply() {
    use mentci_lib::{RenderDatom, RenderOrigin};
    let presented = signal_mentci::QuestionPresented {
        question_identifier: QuestionIdentifier::from("question-1"),
        revision_counter: 1,
        timestamp_nanos: 0,
    };
    let rendered = presented.render_datom(RenderOrigin::Reply);
    assert_eq!(rendered.origin().label(), "reply");
    assert!(rendered.body().contains("question-1"));
}

#[test]
fn view_carries_one_row_per_observed_socket() {
    let mut model = ObservationModel::new(SubscriberName::from("test-client"));
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

#[test]
fn folded_full_projection_surfaces_the_daemon_pane_in_the_view() {
    let mut model = ObservationModel::new(SubscriberName::from("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });

    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            subscription_token: SubscriptionToken::from("subscription-1"),
            projected_interface_state: projected_with_pane(
                question("question-1"),
                pane(
                    "introspect",
                    "(PrototypeWitness (prototype None None None None))",
                ),
                CriomeAccess::ReadWrite,
            ),
        },
    });

    let pane = model.view().pane.expect("a full projection carries a pane");
    assert_eq!(pane.pane_label, "introspect");
    assert!(pane.context_body.contains("PrototypeWitness"));
}

#[test]
fn folded_full_projection_surfaces_criome_access_in_the_view() {
    let mut model = ObservationModel::new(SubscriberName::from("test-client"));
    let _ = model.on_user_event(UserEvent::Observe {
        socket: ComponentSocketKind::Mentci,
        interest: InterfaceInterest::FullInterfaceState,
    });

    model.on_engine_event(EngineEvent::ObservationOpened {
        socket: ComponentSocketKind::Mentci,
        opened: InterfaceObservationOpened {
            subscription_token: SubscriptionToken::from("subscription-1"),
            projected_interface_state: projected_with_access(
                question("question-1"),
                CriomeAccess::ReadWrite,
            ),
        },
    });
    assert_eq!(model.view().criome_access, Some(CriomeAccess::ReadWrite));

    model.on_engine_event(EngineEvent::InterfaceStateChanged {
        socket: ComponentSocketKind::Mentci,
        token: SubscriptionToken::from("subscription-1"),
        state: projected_with_access(question("question-1"), CriomeAccess::ReadOnly),
    });
    assert_eq!(model.view().criome_access, Some(CriomeAccess::ReadOnly));
}
