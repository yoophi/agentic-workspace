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
export type DescribeOutput = Schemas["DescribeOutput"];
export type OperationDescriptor = Schemas["OperationDescriptor"];

// 도메인 DTO alias (038 US1). 프론트 entities의 타입과 필드·표기가 같다.
export type Project = Schemas["ProjectDto"];
export type ProjectCreateInput = Schemas["ProjectCreateInput"];
export type SavedPrompt = Schemas["SavedPromptDto"];
export type Goal = Schemas["GoalDto"];
export type GoalStatus = Schemas["GoalStatus"];
export type AgentRunSettings = Schemas["AgentRunSettingsDto"];
// 038 US2: Git·worktree 조회 결과. 프론트 `entities/project`·`entities/worktree-*`의 타입과 필드·표기가 같다.
export type GitRemote = Schemas["GitRemoteDto"];
export type GitBranch = Schemas["GitBranchDto"];
export type GitWorktree = Schemas["GitWorktreeDto"];
export type WorktreeChange = Schemas["WorktreeChangeDto"];
export type GitWorktreeChanges = Schemas["GitWorktreeChangesDto"];
export type WorktreeFileEntry = Schemas["WorktreeFileEntryDto"];
export type WorktreeTextFile = Schemas["WorktreeTextFileDto"];
export type GitCommitHistory = Schemas["GitCommitHistoryDto"];
export type GitCommitGraph = Schemas["GitCommitGraphDto"];
export type GitCommitDetail = Schemas["GitCommitDetailDto"];
export type GitFileDiff = Schemas["GitFileDiffDto"];
// 038 US3: 새 run 화면의 agent 목록과 이어 붙일 provider 세션.
export type AgentDescriptor = Schemas["AgentDescriptorDto"];
export type ProviderSession = Schemas["ProviderSessionDto"];
// 040: 작업대(Bench)·run·교환.
export type BenchOpenInput = Schemas["BenchOpenInput"];
export type BenchOpenOutput = Schemas["BenchOpenOutput"];
export type BenchCloseOutput = Schemas["BenchCloseOutput"];
export type TitleChangeResult = Schemas["TitleChangeResultDto"];
export type AgentRun = Schemas["AgentRunDto"];
export type AgentRunRequest = Schemas["AgentRunRequestDto"];
export type AgentToolCandidateResponse = Schemas["AgentToolCandidateResponseDto"];
export type AgentExchange = Schemas["AgentExchangeDto"];
export type AgentPanelEndpoint = Schemas["AgentPanelEndpointDto"];
export type AgentWorkspaceSyncResponse = Schemas["AgentWorkspaceSyncResponseDto"];

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

// 039: 이벤트 계약. `EventBySchema`는 스키마 id ↔ typed 본문의 판별 union이다.
export type EventEnvelope = Schemas["EventEnvelope"];
export type EventFrame = Schemas["EventFrame"];
export type EventItem = Schemas["EventItem"];
export type GapNotice = Schemas["GapNotice"];
export type GapReason = Schemas["GapReason"];
export type StreamCursor = Schemas["StreamCursor"];
export type RunEvent = Schemas["RunEventDto"];
export type WorktreeChanged = Schemas["WorktreeChangedDto"];
export type OrchestrationWorkspaceUpdated = Schemas["OrchestrationEventDto"];
export type ExchangeRequested = Schemas["ExchangeRequestedDto"];
export type BenchTitleRequested = Schemas["TitleRequestedDto"];
export type EventBySchema = Schemas["EventBySchema"];
export type EventSchemaId = EventBySchema["schema"];

/** 스키마 id로 typed 봉투를 찾는다. 구독에서 받은 `EventEnvelope`는 `schema`로 좁힌 뒤 이 타입으로 읽는다. */
export type EventMap = {
  [K in EventSchemaId]: Extract<EventBySchema, { schema: K }>;
};

export const PROTOCOL_VERSION = 1 as const;
export const CALLS_PATH = "/v1/calls" as const;
