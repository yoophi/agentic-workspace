use serde_json::json;
use workbench_client::domain::attempt::{Attempt, AttemptError, AttemptState, EndpointIdentity};
use workbench_protocol::{CallReply, CallRequest, FaultCode, OperationId, Outcome, WorkbenchFault};

fn endpoint(epoch: &str) -> EndpointIdentity {
    EndpointIdentity::new("instance-1", epoch).unwrap()
}
fn attempt() -> Attempt {
    Attempt::new(
        CallRequest::command(OperationId::ProjectCreate, json!({"name":"PRIVATE_INPUT"})),
        endpoint("e1"),
    )
    .unwrap()
}

#[test]
fn loss_is_unknown_and_retry_keeps_exact_request_identity() {
    let mut a = attempt();
    let original = a.request().clone();
    let first = a.begin(&endpoint("e1")).unwrap();
    a.mark_unknown(first).unwrap();
    assert_eq!(a.outcome(), Outcome::Unknown);
    let second = a.begin(&endpoint("e1")).unwrap();
    assert_ne!(first, second);
    assert_eq!(a.request(), &original);
    assert_eq!(
        a.complete(first, CallReply::complete(json!({}), None)),
        Err(AttemptError::StaleGeneration)
    );
    let reply = CallReply::complete(json!({"id":"p1"}), Some(4)).mark_replayed();
    a.complete(second, reply.clone()).unwrap();
    assert!(matches!(a.state(), AttemptState::Complete(actual) if actual == &reply));
    assert_eq!(a.outcome(), Outcome::Applied);
}

#[test]
fn epoch_or_instance_change_does_not_reopen_unknown_attempt() {
    let mut a = attempt();
    let first = a.begin(&endpoint("e1")).unwrap();
    a.mark_unknown(first).unwrap();
    for peer in [
        endpoint("e2"),
        EndpointIdentity::new("other-instance", "e1").unwrap(),
    ] {
        assert_eq!(a.begin(&peer), Err(AttemptError::EndpointChanged));
        assert!(matches!(a.state(), AttemptState::Unknown));
    }
    assert_eq!(a.outcome(), Outcome::Unknown);
}

#[test]
fn explicit_fault_keeps_all_fields_and_does_not_infer_not_applied() {
    let mut a = attempt();
    let generation = a.begin(&endpoint("e1")).unwrap();
    let fault = WorkbenchFault::new(
        FaultCode::Unavailable,
        a.request().request_id.clone(),
        "private diagnostic",
    )
    .with_outcome(Outcome::Unknown)
    .with_retryable(true)
    .with_details(json!({"private":"DETAIL_SENTINEL"}));
    a.fail(generation, fault.clone()).unwrap();
    assert!(matches!(a.state(), AttemptState::Fault(actual) if actual == &fault));
    assert_eq!(a.outcome(), Outcome::Unknown);
    assert!(!format!("{a:?}").contains("DETAIL_SENTINEL"));
    assert!(!format!("{a:?}").contains("private diagnostic"));
}

#[test]
fn wrong_fault_identity_leaves_attempt_unresolved() {
    let mut a = attempt();
    let generation = a.begin(&endpoint("e1")).unwrap();
    let fault = WorkbenchFault::new(
        FaultCode::NotFound,
        workbench_protocol::RequestId::random(),
        "other request",
    );
    assert_eq!(a.fail(generation, fault), Err(AttemptError::ReplyIdentity));
    assert_eq!(a.outcome(), Outcome::Unknown);
}

#[test]
fn submitted_and_terminal_attempts_cannot_be_implicitly_restarted() {
    let mut a = attempt();
    let generation = a.begin(&endpoint("e1")).unwrap();
    assert_eq!(a.begin(&endpoint("e1")), Err(AttemptError::InvalidState));
    a.complete(generation, CallReply::complete(json!(null), None))
        .unwrap();
    assert_eq!(a.begin(&endpoint("e1")), Err(AttemptError::InvalidState));
    assert_eq!(a.mark_unknown(generation), Err(AttemptError::InvalidState));
    assert_eq!(a.outcome(), Outcome::Applied);
}

#[test]
fn accepted_reply_is_preserved_without_claiming_final_effect() {
    let mut a = attempt();
    let generation = a.begin(&endpoint("e1")).unwrap();
    let reply = CallReply::Accepted {
        execution_id: "execution1".into(),
        revision: Some(8),
    };
    a.complete(generation, reply.clone()).unwrap();
    assert!(matches!(a.state(), AttemptState::Complete(actual) if actual == &reply));
    assert_eq!(a.outcome(), Outcome::Unknown);
}

#[test]
fn fresh_attempt_debug_does_not_expose_input() {
    let a = attempt();
    assert_eq!(a.outcome(), Outcome::NotApplied);
    assert!(!format!("{a:?}").contains("PRIVATE_INPUT"));
}

#[test]
fn endpoint_and_request_contract_are_checked_before_submission() {
    assert!(EndpointIdentity::new("", "e").is_err());
    assert!(EndpointIdentity::new("i", "").is_err());
    let mut request = CallRequest::command(OperationId::ProjectCreate, json!({}));
    request.idempotency_key = None;
    assert!(Attempt::new(request, endpoint("e1")).is_err());
    let mut request = CallRequest::query(OperationId::ProjectList, json!({}));
    request.protocol_version = 99;
    assert!(Attempt::new(request, endpoint("e1")).is_err());
}

#[test]
fn explicit_fault_retry_preserves_preconditions_but_applied_fault_is_terminal() {
    let mut request = CallRequest::command(OperationId::ProjectUpdate, json!({"id":"p1"}));
    request.expected_revision = Some(42);
    request.timeout_ms = Some(750);
    let mut a = Attempt::new(request.clone(), endpoint("e1")).unwrap();
    let first = a.begin(&endpoint("e1")).unwrap();
    a.fail(
        first,
        WorkbenchFault::new(FaultCode::Unavailable, request.request_id.clone(), "retry")
            .with_outcome(Outcome::Unknown)
            .with_retryable(true),
    )
    .unwrap();
    let second = a.begin(&endpoint("e1")).unwrap();
    assert_eq!(a.request(), &request);
    a.fail(
        second,
        WorkbenchFault::new(FaultCode::Internal, request.request_id, "already applied")
            .with_outcome(Outcome::Applied)
            .with_retryable(true),
    )
    .unwrap();
    assert_eq!(a.outcome(), Outcome::Applied);
    assert_eq!(a.begin(&endpoint("e1")), Err(AttemptError::InvalidState));
}

#[test]
fn nonretryable_fault_retains_server_outcome() {
    let mut a = attempt();
    let generation = a.begin(&endpoint("e1")).unwrap();
    a.fail(
        generation,
        WorkbenchFault::new(
            FaultCode::Forbidden,
            a.request().request_id.clone(),
            "denied",
        ),
    )
    .unwrap();
    assert_eq!(a.outcome(), Outcome::NotApplied);
    assert_eq!(a.begin(&endpoint("e1")), Err(AttemptError::InvalidState));
}

#[test]
fn late_completion_after_local_timeout_cannot_replace_unknown() {
    let mut a = attempt();
    let generation = a.begin(&endpoint("e1")).unwrap();
    a.mark_unknown(generation).unwrap();
    assert_eq!(
        a.complete(generation, CallReply::complete(json!(null), None)),
        Err(AttemptError::InvalidState)
    );
    assert_eq!(a.outcome(), Outcome::Unknown);
}
