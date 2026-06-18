use mentci_lib::approval::{
    ApprovalAnswer, ApprovalContext, ApprovalDecision, ApprovalExplanation, ApprovalIdentifier,
    ApprovalPrompt, ApprovalQuestion, ApprovalResponse, ApprovalSource, SuggestedAnswer,
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
        ApprovalDecision::Answer(ApprovalAnswer::new("Approve after checking mirror head.")),
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
