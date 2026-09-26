//! OpenAPI 3.1 문서 조립(research R1).
//!
//! 개별 타입 스키마는 `utoipa::ToSchema` derive로 얻고, `CallRequest`·`CallReplyByOperation`의 `oneOf`는
//! operation registry(`operations::OPERATIONS`)를 순회해 **프로그램적으로** 만든다. variant마다 `operation`
//! 속성을 단일값 `enum`으로 두어 `openapi-typescript`가 판별 union으로 읽게 하고, `discriminator` object는
//! 쓰지 않는다. 생성물은 `openapi/workbench.openapi.json`에 커밋되며 CI가 drift를 검사한다.

use utoipa::{
    openapi::{
        self,
        path::{HttpMethod, OperationBuilder, PathItem},
        request_body::RequestBodyBuilder,
        schema::{ArrayBuilder, ObjectBuilder, OneOfBuilder, Schema, Type},
        Content, Ref, RefOr, Required, ResponseBuilder, ResponsesBuilder,
    },
    OpenApi,
};

use crate::{
    call::OperationId,
    descriptor::OperationKind,
    operations::{OperationSpec, OPERATIONS},
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Agentic Workbench — Workbench API",
        version = "1",
        description = "Agentic Workbench 서버-클라이언트 Seam의 v1 계약. `POST /v1/calls` 하나로 모든 operation을 호출한다. 이 문서는 `crates/workbench-protocol`의 registry에서 생성된다(수정 금지)."
    ),
    components(schemas(
        crate::call::RequestId,
        crate::call::IdempotencyKey,
        crate::call::CallReply,
        crate::fault::FaultCode,
        crate::fault::Outcome,
        crate::fault::WorkbenchFault,
        crate::principal::Scope,
        crate::descriptor::OperationKind,
        crate::descriptor::Effect,
        crate::descriptor::OperationDescriptor,
        crate::descriptor::DescribeOutput,
        crate::operations::project::ProjectDto,
        crate::operations::project::ProjectListInput,
        crate::operations::project::ProjectCreateInput,
        crate::operations::system::SystemDescribeInput,
        crate::workbench::StreamCursor,
        crate::workbench::Subscription,
        crate::workbench::EventEnvelope,
    ))
)]
struct ApiDoc;

pub const CALL_REQUEST_SCHEMA: &str = "CallRequest";
pub const CALL_REPLY_BY_OPERATION_SCHEMA: &str = "CallReplyByOperation";
pub const CALLS_PATH: &str = "/v1/calls";

fn input_schema_name(id: OperationId) -> &'static str {
    match id {
        OperationId::ProjectList => "ProjectListInput",
        OperationId::ProjectCreate => "ProjectCreateInput",
        OperationId::SystemDescribe => "SystemDescribeInput",
    }
}

fn output_schema(id: OperationId) -> RefOr<Schema> {
    match id {
        OperationId::ProjectList => RefOr::T(Schema::Array(
            ArrayBuilder::new()
                .items(Ref::from_schema_name("ProjectDto"))
                .build(),
        )),
        OperationId::ProjectCreate => Ref::from_schema_name("ProjectDto").into(),
        OperationId::SystemDescribe => Ref::from_schema_name("DescribeOutput").into(),
    }
}

fn variant_title(prefix: &str, id: OperationId) -> String {
    format!("{prefix}_{}", id.as_str().replace('.', "_"))
}

fn operation_literal(id: OperationId) -> ObjectBuilder {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some([id.as_str()]))
}

fn integer(description: &str) -> ObjectBuilder {
    ObjectBuilder::new()
        .schema_type(Type::Integer)
        .description(Some(description))
}

/// `CallRequest` variant: 공통 필드 + `operation` 리터럴 + typed `input`.
fn request_variant(spec: &OperationSpec) -> RefOr<Schema> {
    let mut object = ObjectBuilder::new()
        .title(Some(variant_title("CallRequest", spec.id)))
        .property(
            "protocolVersion",
            integer("generic call/event wire 호환성 축. 037은 1만 지원한다."),
        )
        .property("operation", operation_literal(spec.id))
        .property("requestId", Ref::from_schema_name("RequestId"))
        .property("input", Ref::from_schema_name(input_schema_name(spec.id)))
        .property("idempotencyKey", Ref::from_schema_name("IdempotencyKey"))
        .property(
            "expectedRevision",
            integer("command에서만 의미. aggregate revision과 다르면 preconditionFailed."),
        )
        .property(
            "timeoutMs",
            integer("서버가 상한을 적용하는 상대 시간(ms). 037은 검증만 한다."),
        )
        .required("protocolVersion")
        .required("operation")
        .required("requestId")
        .required("input");
    if spec.kind == OperationKind::Command {
        object = object.required("idempotencyKey");
    }
    RefOr::T(Schema::Object(object.build()))
}

/// typed result: `operation` 리터럴 + `output`(정본 "typed result schema").
fn reply_variant(spec: &OperationSpec) -> RefOr<Schema> {
    let object = ObjectBuilder::new()
        .title(Some(variant_title("CallReply", spec.id)))
        .property(
            "kind",
            ObjectBuilder::new()
                .schema_type(Type::String)
                .enum_values(Some(["complete"])),
        )
        .property("operation", operation_literal(spec.id))
        .property("output", output_schema(spec.id))
        .property(
            "revision",
            integer("command 성공 시 새 aggregate revision. query는 없다."),
        )
        .required("kind")
        .required("operation")
        .required("output");
    RefOr::T(Schema::Object(object.build()))
}

fn one_of(variants: impl IntoIterator<Item = RefOr<Schema>>) -> RefOr<Schema> {
    let mut builder = OneOfBuilder::new();
    for variant in variants {
        builder = builder.item(variant);
    }
    RefOr::T(Schema::OneOf(builder.build()))
}

fn calls_path_item() -> PathItem {
    let request_body = RequestBodyBuilder::new()
        .description(Some("CallRequest — operation별 variant 중 하나"))
        .required(Some(Required::True))
        .content(
            "application/json",
            Content::new(Some(Ref::from_schema_name(CALL_REQUEST_SCHEMA))),
        )
        .build();
    let responses = ResponsesBuilder::new()
        .response(
            "200",
            ResponseBuilder::new()
                .description("CallReply")
                .content(
                    "application/json",
                    Content::new(Some(Ref::from_schema_name("CallReply"))),
                )
                .build(),
        )
        .response(
            "default",
            ResponseBuilder::new()
                .description(
                    "WorkbenchFault (RFC 9457 problem+json). HTTP status는 FaultCode에 따른다.",
                )
                .content(
                    "application/problem+json",
                    Content::new(Some(Ref::from_schema_name("WorkbenchFault"))),
                )
                .build(),
        )
        .build();
    let operation = OperationBuilder::new()
        .operation_id(Some("callWorkbench"))
        .summary(Some("Workbench.call"))
        .description(Some(
            "단일 operation을 호출한다. operation·input·output의 상관관계는 components.schemas.CallRequest / CallReplyByOperation의 oneOf variant로 표현된다.",
        ))
        .request_body(Some(request_body))
        .responses(responses)
        .build();
    PathItem::new(HttpMethod::Post, operation)
}

/// 전체 문서. 결정적 출력을 위해 registry 순서대로 variant를 넣는다.
pub fn build_openapi() -> openapi::OpenApi {
    let mut doc = ApiDoc::openapi();
    let components = doc.components.get_or_insert_with(Default::default);
    components.schemas.insert(
        CALL_REQUEST_SCHEMA.to_owned(),
        one_of(OPERATIONS.iter().map(request_variant)),
    );
    components.schemas.insert(
        CALL_REPLY_BY_OPERATION_SCHEMA.to_owned(),
        one_of(OPERATIONS.iter().map(reply_variant)),
    );
    doc.paths
        .paths
        .insert(CALLS_PATH.to_owned(), calls_path_item());
    doc
}

/// `export_openapi` bin과 golden test가 같은 문자열을 쓴다(끝 개행 포함).
pub fn render_openapi() -> String {
    let mut json = serde_json::to_string_pretty(&build_openapi()).expect("openapi serializes");
    json.push('\n');
    json
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn doc_json() -> Value {
        serde_json::from_str(&render_openapi()).unwrap()
    }

    #[test]
    fn targets_openapi_3_1_and_declares_calls_path() {
        let doc = doc_json();
        assert_eq!(doc["openapi"], "3.1.0");
        assert!(doc["paths"][CALLS_PATH]["post"].is_object());
        assert_eq!(
            doc["paths"][CALLS_PATH]["post"]["operationId"],
            "callWorkbench"
        );
    }

    /// 200 응답이 참조하는 generic `CallReply`는 어떤 operation 결과도 거절하면 안 된다(`project.list`는 배열).
    #[test]
    fn generic_reply_envelope_accepts_any_output_and_uses_camel_case() {
        let doc = doc_json();
        let variants = doc["components"]["schemas"]["CallReply"]["oneOf"]
            .as_array()
            .expect("CallReply oneOf");
        let complete = variants
            .iter()
            .find(|v| v["properties"]["kind"]["enum"] == serde_json::json!(["complete"]))
            .expect("complete variant");
        assert!(
            complete["properties"]["output"].get("type").is_none(),
            "output must be unconstrained JSON, got {}",
            complete["properties"]["output"]
        );
        let accepted = variants
            .iter()
            .find(|v| v["properties"]["kind"]["enum"] == serde_json::json!(["accepted"]))
            .expect("accepted variant");
        assert!(accepted["properties"]["executionId"].is_object());
        assert!(accepted["properties"].get("execution_id").is_none());
        assert_eq!(
            doc["paths"][CALLS_PATH]["post"]["responses"]["200"]["content"]["application/json"]
                ["schema"]["$ref"],
            "#/components/schemas/CallReply"
        );
    }

    #[test]
    fn request_union_has_one_variant_per_operation_with_literal_operation() {
        let doc = doc_json();
        let variants = doc["components"]["schemas"][CALL_REQUEST_SCHEMA]["oneOf"]
            .as_array()
            .expect("oneOf");
        assert_eq!(variants.len(), OPERATIONS.len());
        for (variant, spec) in variants.iter().zip(OPERATIONS.iter()) {
            let literal = &variant["properties"]["operation"]["enum"];
            assert_eq!(literal, &serde_json::json!([spec.id.as_str()]));
            let required = variant["required"].as_array().unwrap();
            assert_eq!(
                required.iter().any(|r| r == "idempotencyKey"),
                spec.kind == OperationKind::Command,
                "{}",
                spec.id
            );
            assert_eq!(
                variant["properties"]["input"]["$ref"],
                format!("#/components/schemas/{}", input_schema_name(spec.id))
            );
        }
    }

    #[test]
    fn reply_union_pairs_each_operation_with_its_output() {
        let doc = doc_json();
        let variants = doc["components"]["schemas"][CALL_REPLY_BY_OPERATION_SCHEMA]["oneOf"]
            .as_array()
            .expect("oneOf");
        assert_eq!(variants.len(), OPERATIONS.len());
        assert_eq!(variants[0]["properties"]["output"]["type"], "array");
        assert_eq!(
            variants[1]["properties"]["output"]["$ref"],
            "#/components/schemas/ProjectDto"
        );
        assert_eq!(
            variants[2]["properties"]["output"]["$ref"],
            "#/components/schemas/DescribeOutput"
        );
    }

    #[test]
    fn rendering_is_deterministic() {
        assert_eq!(render_openapi(), render_openapi());
    }

    /// 커밋된 생성물과 코드가 어긋나면 실패한다. `pnpm run generate:contracts`로 갱신한다.
    #[test]
    fn committed_openapi_matches_registry() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("openapi")
            .join("workbench.openapi.json");
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{} 없음 ({error}). `pnpm run generate:contracts`를 실행해 생성물을 커밋하세요.",
                path.display()
            )
        });
        assert_eq!(
            committed,
            render_openapi(),
            "openapi drift: `pnpm run generate:contracts`를 실행하세요."
        );
    }
}
