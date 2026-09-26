// OperationMap: 생성된 OpenAPI 타입 위에 조건부 타입으로 operation ↔ input/output 상관관계를 고정한다.
// 생성기는 하나(openapi-typescript)뿐이고, 이 파일은 그 출력을 해석만 한다(research R1).
import type { components } from "./generated/workbench";

export type Schemas = components["schemas"];

/** `POST /v1/calls` 요청. operation별 variant의 판별 union. */
export type CallRequest = Schemas["CallRequest"];

/** operation 태그가 붙은 typed 결과 union. */
export type CallReplyByOperation = Schemas["CallReplyByOperation"];

export type CallReply = Schemas["CallReply"];
export type WorkbenchFault = Schemas["WorkbenchFault"];
export type FaultCode = Schemas["FaultCode"];
export type Outcome = Schemas["Outcome"];
export type Project = Schemas["ProjectDto"];
export type ProjectCreateInput = Schemas["ProjectCreateInput"];
export type DescribeOutput = Schemas["DescribeOutput"];
export type OperationDescriptor = Schemas["OperationDescriptor"];

export type OperationId = CallRequest["operation"];

/** operation 이름으로 input·output 타입을 찾는다. 잘못 짝지으면 컴파일 오류다. */
export type OperationMap = {
  [K in OperationId]: {
    input: Extract<CallRequest, { operation: K }>["input"];
    output: Extract<CallReplyByOperation, { operation: K }>["output"];
  };
};

/** 4단계 HTTP/WS Adapter가 구현할 호출 시그니처. */
export type Call = <K extends OperationId>(
  operation: K,
  input: OperationMap[K]["input"],
) => Promise<OperationMap[K]["output"]>;

export const PROTOCOL_VERSION = 1 as const;
export const CALLS_PATH = "/v1/calls" as const;
