//! operation registry의 정적 표. `system.describe`, OpenAPI `oneOf`, authorization이 모두 이 표를 읽는다.

pub mod project;
pub mod system;

use utoipa::PartialSchema;

use crate::{
    call::OperationId,
    descriptor::{Effect, OperationKind},
    principal::Scope,
};

/// operation 하나의 정적 계약(스키마 제외).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSpec {
    pub id: OperationId,
    pub kind: OperationKind,
    pub effect: Effect,
    pub idempotent: bool,
    pub required_scopes: &'static [Scope],
}

pub const OPERATIONS: [OperationSpec; 3] = [
    OperationSpec {
        id: OperationId::ProjectList,
        kind: OperationKind::Query,
        effect: Effect::Read,
        idempotent: false,
        required_scopes: &[Scope::ProjectRead],
    },
    OperationSpec {
        id: OperationId::ProjectCreate,
        kind: OperationKind::Command,
        effect: Effect::Modify,
        idempotent: true,
        required_scopes: &[Scope::ProjectWrite],
    },
    OperationSpec {
        id: OperationId::SystemDescribe,
        kind: OperationKind::Query,
        effect: Effect::Read,
        idempotent: false,
        required_scopes: &[Scope::SystemDescribe],
    },
];

pub fn spec_for(id: OperationId) -> &'static OperationSpec {
    OPERATIONS
        .iter()
        .find(|spec| spec.id == id)
        .expect("every OperationId has a spec")
}

/// operation의 (input, output) JSON Schema. descriptor와 OpenAPI가 공유하는 유일한 출처다.
pub fn schema_for(id: OperationId) -> (serde_json::Value, serde_json::Value) {
    let (input, output) = match id {
        OperationId::ProjectList => (
            project::ProjectListInput::schema(),
            project::project_list_output_schema(),
        ),
        OperationId::ProjectCreate => (
            project::ProjectCreateInput::schema(),
            project::ProjectDto::schema(),
        ),
        OperationId::SystemDescribe => (
            system::SystemDescribeInput::schema(),
            crate::descriptor::DescribeOutput::schema(),
        ),
    };
    (
        serde_json::to_value(input).expect("schema serializes"),
        serde_json::to_value(output).expect("schema serializes"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_operation_has_a_spec_with_matching_id() {
        for id in OperationId::ALL {
            assert_eq!(spec_for(id).id, id);
        }
        assert!(spec_for(OperationId::ProjectCreate).idempotent);
        assert_eq!(
            spec_for(OperationId::ProjectCreate).kind,
            OperationKind::Command
        );
        assert!(!spec_for(OperationId::ProjectList).idempotent);
    }

    #[test]
    fn schemas_are_objects_for_every_operation() {
        for id in OperationId::ALL {
            let (input, output) = schema_for(id);
            assert!(input.is_object(), "{id} input");
            assert!(output.is_object(), "{id} output");
        }
        let (_, list_output) = schema_for(OperationId::ProjectList);
        assert_eq!(list_output["type"], "array");
    }
}
