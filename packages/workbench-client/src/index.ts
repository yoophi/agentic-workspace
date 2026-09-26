// 037: 생성 타입과 OperationMap만 노출한다. 어떤 앱도 아직 이 패키지를 import하지 않는다.
// HTTP/WebSocket Adapter는 4단계(Desktop 전환)에서 추가된다.
export type * from "./generated/workbench";
export type {
  Call,
  CallReply,
  CallReplyByOperation,
  CallRequest,
  DescribeOutput,
  FaultCode,
  OperationDescriptor,
  OperationId,
  OperationMap,
  Outcome,
  Project,
  ProjectCreateInput,
  Schemas,
  WorkbenchFault,
} from "./operation-map";
export { CALLS_PATH, PROTOCOL_VERSION } from "./operation-map";
