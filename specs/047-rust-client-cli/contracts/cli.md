# `aw` machine command 계약

`aw events watch --input -`, `aw operations [operation]`, `aw call OPERATION --input -`, `aw project list`, `aw run start --input -`, `aw run watch RUN --after N`, `aw run cancel RUN --idempotency-key KEY`, `aw server status`. 편의 command는 generic call과 동일 policy/authority/typed schema. 실제 production activation은 client 계약 gate, pending operation은 조용히 launch하지 않음.

finite JSON: 성공 stdout JSON1개 `{ok:true,data,requestId,revision?,replayed?}`+newline; 실패 stdout0, stderr JSON1개 `{ok:false,error:{code,outcome,retryable,...safeFields},requestId?,attempt?}`+newline. library는 full fault details를 보존하지만 console은 credential/private arbitrary details를 덤프하지 않고 safe code/outcome/scope와 필요한 bounded retry identity만 반환. human logs는 stderr, machine spinner/color/log0. requestId는 trace, key는 mutation retry; unknown 오류는 private retry-state 위치/attempt identity를 안전한 필드로 반환하고 다음 invocation은 `--retry-state`의 원 input/key/epoch를 사용한다. mutation 첫 전송 전 state durable publish가 실패하면 요청0. raw payload/private fingerprint는 console로 노출하지 않음.

exit:0success/1internal/2usage-schema-unsupported/3auth-forbidden/4notfound/5conflict-precondition/6cancel-rejected/7deadline/8unavailable/9version/10interaction/130SIGINT. outcome unknown은 error envelope로 보존하고 별도 exit family로 적용 상태를 단정하지 않음. 최신 047 완료 범위에서 cancel-rejected6은 RunCancel production prerequisite가 미충족하여 미검증/후속 이연이다. explicit/generic cancel 모두 prerequisiteUnavailable8을 유지하고 typed cancel input은 보존하며 exit6 proof를 만들기 위해 gate를 해제하지 않는다. 도달 가능한 나머지 exit family와 가능한 047 계약/actual server 검증은 필수다. input1MiB와 EOF/UTF8/strictschema 확인, token argv 금지. future destructive grant는 existingserver 계약만 사용, --yes로 grant생성0.

JSONL: open 전 finite 오류; open 뒤 stream.open→event/control records→stream.end. 각 newline까지 write_all 성공해야 cursor ACK. stdout write error/broken pipe는 ACK 없이 local 종료; server cancel 없음. SIGINT final bounded stream.end 가능한 경우만 쓰고 exit130, old stream generation 완료가 다음 요청에 mutation0. private logs/ticket/token/protocol frame raw debug0.

actual mcp serve/TUI renderer/packaging은 원 roadmap 별도이며 새 client가 그 완료를 의미하지 않음.

## Generic event watch (D-C4)

`aw events watch --input -`는 bounded stdin JSON `{streamId, epoch, afterSequence}`를 읽는 streaming 명령이다. streamId는 서버가 반환한 완전한 주소이며, epoch는 검증된 서버 epoch, afterSequence는 마지막 적용 완료 위치의 nonnegative integer다. 미지 stream kind/잘못된 cursor/epoch 불일치는 open 전에 typed finite 오류로 거절한다. secret/token/ticket은 stdin schema에도 포함하지 않고 기존 verified endpoint credential provider만 사용한다. 원 서버 scope/소유권 및 production admission을 동일하게 적용하며 arbitrary subscribe가 agent authority를 확대하지 않는다. `aw run watch`도 같은 event consumer/JSONL 출력/ACK 구현을 사용한다.

실제 통합 harness는 bootstrap 응답 eventStreamId와 verified epoch, afterSequence=0을 이 명령의 stdin에 전달해 **실제 aw subprocess**를 실행한다. harness가 stdout의 완전한 stream.open 및 bootstrap event JSONL을 읽은 뒤 private empty recover를 제출한다. subprocess의 write_all(newline 포함) 성공을 ACK로 사용하고, runtimeReconciled(s+1,r+1)와 notificationRecovery(s+2,r+1) 두 JSONL의 streamId/epoch/reason/sequence/revision을 검증한다. stdout 관찰은 단순 library callback으로 대체하지 않는다. controlled consumer 시험은 partial write/broken pipe/old generation에서 ACK0 및 cursor 미진전을 별도로 검증한다. 두 이벤트·HTTP reply·최종 snapshot 비교 뒤 SIGINT를 보내 가능한 stream.end와 exit130 및 bounded child reap를 확인한다. 읽기/종료 deadline 만료 또는 출력 누락은 시험 실패다.
