# Caller 데이터 모델

- VerifiedEndpoint: validated loopback URL, instanceId/serverEpoch/protocol/storage, bound credential provider. constructor는 private verified adapter 또는 test-only fixture. caller bool로 production readiness를 만들 수 없음.
- CallerProfile: Owner와AgentScoped를 구분; authority/expiry/bench/run/generation bound. secret Debug/Serialize 없음. agent에 owner fallback 없음.
- CallAttempt: operation/requestId/key/normalized immutable input fingerprint/instance+epoch/operationGeneration, Submitted→Complete 또는Fault 또는Unknown. Unknown 뒤 local cancellation은 server NotApplied가 아님. same payload/key/instance retry만 가능.
- CallOutcome: full CallReply/WorkbenchFault와 TransportUnknown·ProtocolViolation·PrerequisiteUnavailable 구분. HTTP/body/request/operation/epoch consistency 검사. details는 library 보존, console 안전 projection.
- AppliedCursor: streamId/serverEpoch/consumerGeneration/sequence. received position 별도. consumer ACK·stdout complete write 이후만 increment, old generation completion 무시.
- Recovery: Connecting→Live→Reconnecting; gap→LiveBuffering→SnapshotLoading→ConsumerReset→FilteredReplay→Live. snapshot/hello 실패 또는 newgap는 fresh operationGeneration에서 재시도, max attempts exhausted typed failure.
- Limits: nonzero input/body/frame/queue byte+item/whole deadline, cancellation outstanding task ownership.

credential/private payload와 process handle/storage data를 model diagnostics에 넣지 않는다. TUI·CLI는 cursor와 소비 callback을 각 caller가 소유하며 global singleton reducer를 공유하지 않는다.

PrivateRetryState는 caller control store의 versioned immutable attempt와 CAS outcome이며 원 input bytes는 owner-only file에만 유지한다. pre-send publish 실패는 request0, unknown 재시도는 동일 state/key/instance/epoch, old completion CAS loser가 새 operation을 수정하지 못한다. VerifiedConnection은 owned TCP HTTP sender+driver를 함께 소유하고 credentials를 보낼 수 있는 state는 그 socket의 identify 성공으로만 얻는다. socket 교체는 proof를 무효화하며 WS upgrade에도 같은 ownership이 적용된다.
