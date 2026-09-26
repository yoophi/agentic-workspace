//! `project.*` operation의 input/output wire 타입.

use serde::{Deserialize, Serialize};
use utoipa::{
    openapi::{schema::ArrayBuilder, RefOr, Schema},
    PartialSchema, ToSchema,
};

/// 프론트 `entities/project/model/types.ts`의 `Project`와 필드·표기가 같아야 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub working_directory: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// `project.list` input. 필드가 없고 추가 필드는 거절한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectListInput {}

pub type ProjectListOutput = Vec<ProjectDto>;

/// `Vec<ProjectDto>`의 스키마. `schema_for`가 output에 쓴다.
pub fn project_list_output_schema() -> RefOr<Schema> {
    RefOr::T(Schema::Array(
        ArrayBuilder::new().items(ProjectDto::schema()).build(),
    ))
}

/// `project.create` input. 정규화(trim 등)는 core의 `project_service`가 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectCreateInput {
    pub name: String,
    pub working_directory: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub type ProjectCreateOutput = ProjectDto;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn project_dto_matches_frontend_shape() {
        let dto = ProjectDto {
            id: "project-1".into(),
            name: "AW".into(),
            working_directory: "/tmp/aw".into(),
            description: None,
        };
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            json!({"id": "project-1", "name": "AW", "workingDirectory": "/tmp/aw", "description": null})
        );
    }

    #[test]
    fn list_input_rejects_unknown_fields() {
        assert!(serde_json::from_value::<ProjectListInput>(json!({})).is_ok());
        let err = serde_json::from_value::<ProjectListInput>(json!({"foo": 1})).unwrap_err();
        assert!(err.to_string().contains("foo"));
    }

    #[test]
    fn create_input_requires_name_and_directory_fields() {
        let err = serde_json::from_value::<ProjectCreateInput>(json!({"name": "x"})).unwrap_err();
        assert!(err.to_string().contains("workingDirectory"));
        let ok: ProjectCreateInput =
            serde_json::from_value(json!({"name": "x", "workingDirectory": "/y"})).unwrap();
        assert_eq!(ok.description, None);
    }

    #[test]
    fn list_output_schema_is_array_of_projects() {
        let schema = serde_json::to_value(project_list_output_schema()).unwrap();
        assert_eq!(schema["type"], "array");
        assert!(schema["items"].is_object());
    }
}
