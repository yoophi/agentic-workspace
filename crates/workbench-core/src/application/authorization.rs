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

    /// 044: 소유자 전용 operation(`server:read`·`server:admin`).
    const OWNER_ONLY: [OperationId; 7] = [
        OperationId::ServerStatus,
        OperationId::ServerStop,
        OperationId::LeaseAcquire,
        OperationId::LeaseRenew,
        OperationId::LeaseRelease,
        OperationId::DesktopIssueWindowToken,
        OperationId::DesktopRetireWindow,
    ];

    #[test]
    fn desktop_may_call_everything_except_owner_only_operations() {
        let desktop = AuthenticatedPrincipal::desktop();
        for id in OperationId::ALL {
            let resolved = resolve_operation(&rid(), &desktop, id.as_str());
            if OWNER_ONLY.contains(&id) {
                assert_eq!(resolved.unwrap_err().code, FaultCode::Forbidden, "{id}");
            } else {
                assert_eq!(resolved.unwrap(), id);
            }
        }
        let expected: Vec<OperationId> = OperationId::ALL
            .into_iter()
            .filter(|id| !OWNER_ONLY.contains(id))
            .collect();
        assert_eq!(visible_operations(&desktop), expected);
        // 창 주체도 같다(창 토큰으로 서버 정지·토큰 발급을 할 수 없다).
        assert_eq!(
            visible_operations(&AuthenticatedPrincipal::desktop_window("w", "i")),
            expected
        );
    }

    #[test]
    fn owner_may_call_everything() {
        let owner = AuthenticatedPrincipal::owner();
        for id in OperationId::ALL {
            assert_eq!(resolve_operation(&rid(), &owner, id.as_str()).unwrap(), id);
        }
        assert_eq!(visible_operations(&owner), OperationId::ALL.to_vec());
    }

    #[test]
    fn readonly_is_forbidden_from_create_and_does_not_see_it() {
        let readonly = AuthenticatedPrincipal::test_readonly();
        let fault = resolve_operation(&rid(), &readonly, "project.create").unwrap_err();
        assert_eq!(fault.code, FaultCode::Forbidden);
        let visible = visible_operations(&readonly);
        assert!(visible.contains(&OperationId::ProjectList));
        assert!(visible.contains(&OperationId::SystemDescribe));
        assert!(!visible.contains(&OperationId::ProjectCreate));
        // 조회만 보인다: 표의 Query 수에서 소유자 전용 조회(`server.status`)를 뺀 수와 같다.
        let queries = OPERATIONS
            .iter()
            .filter(|spec| spec.kind == workbench_protocol::OperationKind::Query)
            .filter(|spec| !OWNER_ONLY.contains(&spec.id))
            .count();
        assert_eq!(visible.len(), queries);
        assert!(!visible.contains(&OperationId::ServerStatus));
    }

    #[test]
    fn unknown_operation_is_not_found_even_for_desktop() {
        let fault = resolve_operation(&rid(), &AuthenticatedPrincipal::desktop(), "project.rename")
            .unwrap_err();
        assert_eq!(fault.code, FaultCode::NotFound);
        assert!(fault.message.contains("project.rename"));
    }
}
