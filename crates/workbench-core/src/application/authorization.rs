//! operation 존재·권한 판정. `system.describe`와 `call`이 같은 함수를 쓴다(정본 Invariant 2).

use workbench_protocol::{
    operations::{spec_for, OPERATIONS},
    AuthenticatedPrincipal, OperationId, RequestId, Scope, WorkbenchFault,
};

pub fn required_scopes(operation: OperationId) -> &'static [Scope] {
    spec_for(operation).required_scopes
}

/// 이름을 `OperationId`로 해석하고 권한을 검사한다. 미존재 → `notFound`, scope 부족 → `forbidden`.
pub fn resolve_operation(
    request_id: &RequestId,
    principal: &AuthenticatedPrincipal,
    operation: &str,
) -> Result<OperationId, WorkbenchFault> {
    let id = OperationId::parse(operation)
        .ok_or_else(|| WorkbenchFault::not_found(request_id.clone(), operation))?;
    if !principal.has_all(required_scopes(id)) {
        return Err(WorkbenchFault::forbidden(request_id.clone(), operation));
    }
    Ok(id)
}

/// principal에게 허용된 operation만, 정적 표 순서대로.
pub fn visible_operations(principal: &AuthenticatedPrincipal) -> Vec<OperationId> {
    OPERATIONS
        .iter()
        .filter(|spec| principal.has_all(spec.required_scopes))
        .map(|spec| spec.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use workbench_protocol::FaultCode;

    use super::*;

    fn rid() -> RequestId {
        RequestId::new("r1").unwrap()
    }

    #[test]
    fn desktop_may_call_everything() {
        let desktop = AuthenticatedPrincipal::desktop();
        for id in OperationId::ALL {
            assert_eq!(
                resolve_operation(&rid(), &desktop, id.as_str()).unwrap(),
                id
            );
        }
        assert_eq!(visible_operations(&desktop), OperationId::ALL.to_vec());
    }

    #[test]
    fn readonly_is_forbidden_from_create_and_does_not_see_it() {
        let readonly = AuthenticatedPrincipal::test_readonly();
        let fault = resolve_operation(&rid(), &readonly, "project.create").unwrap_err();
        assert_eq!(fault.code, FaultCode::Forbidden);
        assert_eq!(
            visible_operations(&readonly),
            vec![OperationId::ProjectList, OperationId::SystemDescribe]
        );
    }

    #[test]
    fn unknown_operation_is_not_found_even_for_desktop() {
        let fault = resolve_operation(&rid(), &AuthenticatedPrincipal::desktop(), "project.rename")
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::NotFound);
        assert!(fault.message.contains("project.rename"));
    }
}
