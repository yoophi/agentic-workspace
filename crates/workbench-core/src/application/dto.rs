//! 도메인 타입 ↔ protocol wire DTO 변환(research R1).
//!
//! DTO는 protocol crate에 **미러**로 정의되고 여기서 변환한다. wire가 이전(AW가 도메인 타입을 그대로 직렬화)과
//! 같음은 `assert_wire_parity`가 고정한다: 도메인 값을 serde로 직렬화한 JSON == DTO를 직렬화한 JSON.

use workbench_protocol::operations::project::ProjectDto;

use crate::domain::project::Project;

pub fn to_dto(project: &Project) -> ProjectDto {
    ProjectDto {
        id: project.id.clone(),
        name: project.name.clone(),
        working_directory: project.working_directory.clone(),
        description: project.description.clone(),
    }
}

/// 테스트 helper: 도메인 값과 DTO의 JSON이 같은지 확인한다. 다르면 두 JSON을 함께 보여 준다.
#[cfg(test)]
pub(crate) fn assert_wire_parity<D: serde::Serialize, W: serde::Serialize>(
    label: &str,
    domain: &D,
    dto: &W,
) {
    let domain_json = serde_json::to_value(domain).expect("domain serializes");
    let dto_json = serde_json::to_value(dto).expect("dto serializes");
    assert_eq!(
        domain_json,
        dto_json,
        "{label}: wire mismatch\n domain = {}\n dto    = {}",
        serde_json::to_string_pretty(&domain_json).unwrap(),
        serde_json::to_string_pretty(&dto_json).unwrap()
    );
}

/// 테스트 helper: 프론트가 보내는 형태의 JSON이 input DTO로 역직렬화되고 다시 같은 JSON이 되는지 확인한다.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn assert_input_roundtrip<I: serde::de::DeserializeOwned + serde::Serialize>(
    label: &str,
    json: serde_json::Value,
) -> I {
    let parsed: I = serde_json::from_value(json.clone())
        .unwrap_or_else(|error| panic!("{label}: input does not deserialize: {error}\n{json}"));
    let back = serde_json::to_value(&parsed).expect("input serializes");
    assert_eq!(back, json, "{label}: input roundtrip changed the JSON");
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_wire_parity() {
        for description in [None, Some("desc".to_owned())] {
            let project = Project {
                id: "project-1".into(),
                name: "AW".into(),
                working_directory: "/tmp/aw".into(),
                description,
            };
            assert_wire_parity("project", &project, &to_dto(&project));
        }
    }
}
