use mentci_lib::approval::{
    AnswerBody, AnswerProposal, ApprovalClientIdentifier, ApprovalContext, ApprovalDecision,
    ApprovalDelivery, ApprovalExplanation, ApprovalIdentifier, ApprovalInterest, ApprovalPrompt,
    ApprovalQuestion, ApprovalResponse, ApprovalSource, ApprovalState, ApprovalUpdate,
    SuggestedAnswer,
};
use mentci_lib::{Cmd, EngineEvent, UserEvent, WorkbenchState};
use signal::{Principal, Slot};

fn approval_question(identifier: u64) -> ApprovalQuestion {
    ApprovalQuestion::new(
        ApprovalIdentifier::new(identifier),
        ApprovalSource::CriomeEscalation,
        ApprovalPrompt::new("Should this object update be accepted?"),
        SuggestedAnswer::new("Approve"),
        ApprovalExplanation::new("The requested head is quorum-signed."),
        ApprovalContext::new("Spirit on another node is waiting for the approved head."),
    )
}

#[test]
fn incoming_approval_question_becomes_current_and_notifies_runtime() {
    let mut workbench = WorkbenchState::new(Slot::<Principal>::from(1));
    let question = approval_question(7);

    let commands = workbench.on_engine_event(EngineEvent::ApprovalQuestionArrived {
        question: question.clone(),
    });
    let view = workbench.view();

    assert_eq!(view.approval.pending_count, 1);
    assert_eq!(view.approval.answered_count, 0);
    assert_eq!(view.approval.current, Some(question.clone()));
    match commands.as_slice() {
        [Cmd::NotifyApproval { question: notified }] => {
            assert_eq!(notified, &question);
        }
        other => panic!("expected one NotifyApproval command, got {other:?}"),
    }
}

#[test]
fn answering_approval_question_removes_it_and_submits_response() {
    let mut workbench = WorkbenchState::new(Slot::<Principal>::from(1));
    let question = approval_question(9);
    workbench.on_engine_event(EngineEvent::ApprovalQuestionArrived {
        question: question.clone(),
    });
    let response = ApprovalResponse::new(
        question.identifier,
        ApprovalDecision::ApproveSuggestedAnswer,
    );

    let commands = workbench.on_user_event(UserEvent::AnswerApproval {
        response: response.clone(),
    });
    let view = workbench.view();

    assert_eq!(view.approval.pending_count, 0);
    assert_eq!(view.approval.answered_count, 1);
    assert_eq!(view.approval.current, None);
    match commands.as_slice() {
        [
            Cmd::SubmitApproval {
                response: submitted,
            },
        ] => {
            assert_eq!(submitted, &response);
        }
        other => panic!("expected one SubmitApproval command, got {other:?}"),
    }
}

#[test]
fn edited_approval_answer_is_submitted_as_a_new_proposal() {
    let mut workbench = WorkbenchState::new(Slot::<Principal>::from(1));
    let question = approval_question(10);
    workbench.on_engine_event(EngineEvent::ApprovalQuestionArrived {
        question: question.clone(),
    });
    let proposal = AnswerProposal::new(
        question.identifier,
        AnswerBody::new("Approve after checking mirror head."),
    );

    let commands = workbench.on_user_event(UserEvent::ProposeApprovalAnswer {
        proposal: proposal.clone(),
    });
    let view = workbench.view();

    assert_eq!(view.approval.pending_count, 1);
    assert_eq!(view.approval.answered_count, 0);
    assert_eq!(view.approval.current, Some(question));
    match commands.as_slice() {
        [
            Cmd::SubmitAnswerProposal {
                proposal: submitted,
            },
        ] => {
            assert_eq!(submitted, &proposal);
        }
        other => panic!("expected one SubmitAnswerProposal command, got {other:?}"),
    }
}

#[test]
fn deferring_approval_question_keeps_it_pending() {
    let mut workbench = WorkbenchState::new(Slot::<Principal>::from(1));
    let question = approval_question(11);
    workbench.on_engine_event(EngineEvent::ApprovalQuestionArrived {
        question: question.clone(),
    });

    let commands = workbench.on_user_event(UserEvent::AnswerApproval {
        response: ApprovalResponse::new(question.identifier, ApprovalDecision::Defer),
    });
    let view = workbench.view();

    assert_eq!(view.approval.pending_count, 1);
    assert_eq!(view.approval.answered_count, 0);
    assert_eq!(view.approval.current, Some(question));
    assert!(commands.is_empty());
}

#[test]
fn approval_state_subscription_receives_initial_snapshot() {
    let mut approval = ApprovalState::default();
    let question = approval_question(13);
    approval.receive(question.clone());

    let receipt = approval.subscribe(ApprovalClientIdentifier::new(3), ApprovalInterest::All);

    assert_eq!(
        receipt.subscription.client,
        ApprovalClientIdentifier::new(3)
    );
    assert_eq!(receipt.snapshot.pending_count, 1);
    assert_eq!(receipt.snapshot.answered_count, 0);
    assert_eq!(receipt.snapshot.subscription_count, 1);
    assert_eq!(receipt.snapshot.current, Some(question));
}

#[test]
fn approval_state_broadcasts_question_to_subscribers() {
    let mut approval = ApprovalState::default();
    let receipt = approval.subscribe(ApprovalClientIdentifier::new(5), ApprovalInterest::All);
    let question = approval_question(15);

    let deliveries = approval.receive(question.clone());

    assert_eq!(
        deliveries,
        vec![ApprovalDelivery::new(
            receipt.subscription.identifier,
            ApprovalClientIdentifier::new(5),
            ApprovalUpdate::QuestionReceived(question)
        )]
    );
}

#[test]
fn workbench_publishes_daemon_state_updates_to_approval_subscribers() {
    let mut workbench = WorkbenchState::new(Slot::<Principal>::from(1));
    let subscription_commands = workbench.on_user_event(UserEvent::SubscribeApproval {
        client: ApprovalClientIdentifier::new(8),
        interest: ApprovalInterest::All,
    });
    let receipt = match subscription_commands.as_slice() {
        [Cmd::ConfirmApprovalSubscription { receipt }] => receipt.clone(),
        other => panic!("expected one subscription receipt, got {other:?}"),
    };
    let question = approval_question(17);

    let commands = workbench.on_engine_event(EngineEvent::ApprovalQuestionArrived {
        question: question.clone(),
    });

    match commands.as_slice() {
        [
            Cmd::NotifyApproval { question: notified },
            Cmd::PublishApprovalUpdates { deliveries },
        ] => {
            assert_eq!(notified, &question);
            assert_eq!(
                deliveries,
                &vec![ApprovalDelivery::new(
                    receipt.subscription.identifier,
                    ApprovalClientIdentifier::new(8),
                    ApprovalUpdate::QuestionReceived(question)
                )]
            );
        }
        other => panic!("expected notification and approval-state delivery, got {other:?}"),
    }
}

#[test]
fn selecting_approval_question_broadcasts_daemon_state_update() {
    let mut workbench = WorkbenchState::new(Slot::<Principal>::from(1));
    let subscription_commands = workbench.on_user_event(UserEvent::SubscribeApproval {
        client: ApprovalClientIdentifier::new(13),
        interest: ApprovalInterest::PendingQuestions,
    });
    let receipt = match subscription_commands.as_slice() {
        [Cmd::ConfirmApprovalSubscription { receipt }] => receipt.clone(),
        other => panic!("expected one subscription receipt, got {other:?}"),
    };
    let first = approval_question(31);
    let second = approval_question(32);
    workbench.on_engine_event(EngineEvent::ApprovalQuestionArrived { question: first });
    workbench.on_engine_event(EngineEvent::ApprovalQuestionArrived {
        question: second.clone(),
    });

    let commands = workbench.on_user_event(UserEvent::SelectApproval {
        identifier: second.identifier,
    });

    match commands.as_slice() {
        [Cmd::PublishApprovalUpdates { deliveries }] => {
            assert_eq!(
                deliveries,
                &vec![ApprovalDelivery::new(
                    receipt.subscription.identifier,
                    ApprovalClientIdentifier::new(13),
                    ApprovalUpdate::QuestionSelected(second.identifier)
                )]
            );
        }
        other => panic!("expected approval-state delivery, got {other:?}"),
    }
}

#[test]
fn answered_response_broadcasts_to_answer_subscriber() {
    let mut workbench = WorkbenchState::new(Slot::<Principal>::from(1));
    let subscription_commands = workbench.on_user_event(UserEvent::SubscribeApproval {
        client: ApprovalClientIdentifier::new(21),
        interest: ApprovalInterest::AnsweredResponses,
    });
    let receipt = match subscription_commands.as_slice() {
        [Cmd::ConfirmApprovalSubscription { receipt }] => receipt.clone(),
        other => panic!("expected one subscription receipt, got {other:?}"),
    };
    let question = approval_question(23);
    workbench.on_engine_event(EngineEvent::ApprovalQuestionArrived {
        question: question.clone(),
    });
    let response = ApprovalResponse::new(
        question.identifier,
        ApprovalDecision::ApproveSuggestedAnswer,
    );

    let commands = workbench.on_user_event(UserEvent::AnswerApproval {
        response: response.clone(),
    });

    match commands.as_slice() {
        [
            Cmd::SubmitApproval {
                response: submitted,
            },
            Cmd::PublishApprovalUpdates { deliveries },
        ] => {
            assert_eq!(submitted, &response);
            assert_eq!(
                deliveries,
                &vec![ApprovalDelivery::new(
                    receipt.subscription.identifier,
                    ApprovalClientIdentifier::new(21),
                    ApprovalUpdate::QuestionAnswered(response)
                )]
            );
        }
        other => panic!("expected submission and approval-state delivery, got {other:?}"),
    }
}
