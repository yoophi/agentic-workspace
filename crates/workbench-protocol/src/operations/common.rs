//! operation 계약이 공유하는 스키마 조각.

use serde::{Deserialize, Serialize};
use utoipa::{
    openapi::{
        schema::{ArrayBuilder, ObjectBuilder, OneOfBuilder, Schema, Type},
        RefOr,
    },
    PartialSchema, ToSchema,
};

/// 출력이 없는 변경(`project.delete`, `git.createWorktree` 등)의 output. wire에서는 `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EmptyOutput;

impl PartialSchema for EmptyOutput {
    fn schema() -> RefOr<Schema> {
        null_schema()
    }
}

impl ToSchema for EmptyOutput {}

pub fn null_schema() -> RefOr<Schema> {
    RefOr::T(Schema::Object(
        ObjectBuilder::new().schema_type(Type::Null).build(),
    ))
}

/// `T | null` — `goal.get`·`agentRunSettings.get`처럼 없을 수 있는 단일 항목 출력.
pub fn nullable_schema(inner: RefOr<Schema>) -> RefOr<Schema> {
    RefOr::T(Schema::OneOf(
        OneOfBuilder::new().item(inner).item(null_schema()).build(),
    ))
}

/// `T[]` — 목록 출력.
pub fn array_schema(items: RefOr<Schema>) -> RefOr<Schema> {
    RefOr::T(Schema::Array(ArrayBuilder::new().items(items).build()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_output_is_null_on_the_wire_and_in_schema() {
        assert_eq!(
            serde_json::to_value(EmptyOutput).unwrap(),
            serde_json::Value::Null
        );
        let parsed: EmptyOutput = serde_json::from_value(serde_json::Value::Null).unwrap();
        assert_eq!(parsed, EmptyOutput);
        let schema = serde_json::to_value(EmptyOutput::schema()).unwrap();
        assert_eq!(schema["type"], "null");
    }

    #[test]
    fn nullable_wraps_inner_in_one_of_with_null() {
        let schema = serde_json::to_value(nullable_schema(null_schema())).unwrap();
        let one_of = schema["oneOf"].as_array().unwrap();
        assert_eq!(one_of.len(), 2);
        assert_eq!(one_of[1]["type"], "null");
    }

    #[test]
    fn array_schema_has_items() {
        let schema = serde_json::to_value(array_schema(null_schema())).unwrap();
        assert_eq!(schema["type"], "array");
        assert!(schema["items"].is_object());
    }
}
