# `aw` machine command 계약

`aw operations [operation]`, `aw call OPERATION --input -`, `aw project list`, `aw run start --input -`, `aw run watch RUN --after N`, `aw run cancel RUN --idempotency-key KEY`, `aw server status`. 편의 command는 generic call과 동일 policy/authority/typed schema. 실제 production activation은 client 계약 gate, pending operation은 조용히 launch하지 않음.

finite JSON: 성공 stdout JSON1개 `{ok:true,data,requestId,revision?,replayed?}`+newline; 실패 stdout0, stderr JSON1개 `{ok:false,error:{code,outcome,retryable,...safeFields},requestId?,attempt?}`+newline. library는 full fault details를 보존하지만 console은 credential/private arbitrary details를 덤프하지 않고 safe code/outcome/scope와 필요한 bounded retry identity만 반환. human logs는 stderr, machine spinner/color/log0. requestId는 trace, key는 mutation retry; 자동 생성 key는 unknown 오류에 반환하여 사용자 retry에서 유지할 수 있게 한다. raw payload/private fingerprint는 console로 노출하지 않음.

exit:0success/1internal/2usage-schema-unsupported/3auth-forbidden/4notfound/5conflict-precondition/6cancel-rejected/7deadline/8unavailable/9version/10interaction/130SIGINT. outcome unknown은 error envelope로 보존하고 별도 exit family로 적용 상태를 단정하지 않음. input1MiB와 EOF/UTF8/strictschema 확인, token argv 금지. future destructive grant는 existingserver 계약만 사용, --yes로 grant생성0.

JSONL: open 전 finite 오류; open 뒤 stream.open→event/control records→stream.end. 각 newline까지 write_all 성공해야 cursor ACK. stdout write error/broken pipe는 ACK 없이 local 종료; server cancel 없음. SIGINT final bounded stream.end 가능한 경우만 쓰고 exit130, old stream generation 완료가 다음 요청에 mutation0. private logs/ticket/token/protocol frame raw debug0.

actual mcp serve/TUI renderer/packaging은 원 roadmap 별도이며 새 client가 그 완료를 의미하지 않음.
