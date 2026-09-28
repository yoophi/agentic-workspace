# Contract: Platform containment

## macOS / Linux

- composition root는 normal startup 전에 숨은 keeper mode를 판별한다.
- keeper는 payload를 새 process group에 spawn하고 256-bit attempt nonce를 launch identity로 주입한다.
- keeper가 payload PID/start identity/nonce handshake를 완료하고 target inventory에서 동일 identity를 확인하기 전 server는 Adopted로 보지 않는다.
- keeper는 같은 uid, 동일 nonce, launch 이후 start identity를 모두 만족하는 live process 집합을 유지한다. PID, PPID 또는 process group 하나만으로 소유를 판정하지 않는다.
- server control pipe EOF, cancel, timeout, shutdown에서 keeper가 원 group graceful → timeout → force를 수행한 뒤, 집합에 남은 이탈 process를 매 signal 직전 start identity 재검증 후 종료한다.
- cleanup complete는 (a) 원 group에 live process 0, (b) nonce+start identity live set 0이 연속 quiescence window 동안 유지, (c) direct payload/keeper wait 완료를 모두 만족할 때다.
- leader가 먼저 끝나도 keeper는 live set이 0이 되기 전 완료하지 않는다.
- 새 process group/session, double-fork+reparent, inherited control FD close fixture가 장기 실행한 뒤 server/keeper parent가 종료돼도 live set이 0으로 수렴해야 한다.
- target inventory가 이탈 descendant identity를 증명하지 못하면 group kill 결과를 성공으로 반환하지 않고 containment capability unavailable로 fail-closed한다. 이 상태에서는 045 platform gate가 실패한다.

## Unix에서 아직 검증할 가정

- descendant가 `env_clear` 또는 새 `execve` environment로 nonce를 제거한 뒤에도 추적 가능한 OS identity 수단이 필요하다. nonce 상속만 통과하면 전체 containment 증거가 아니다.
- 같은 uid process environment/identity inventory가 실제 app sandbox·hardened runtime·배포 권한에서 허용되어야 한다.
- identity 확인과 signal 사이에는 재사용 불가능한 handle 또는 동등한 원자적 검증이 필요하다. `pid + start time 확인 → 나중에 kill(pid)`만으로 통과할 수 없다.
- keeper 자체 hard kill에서는 다음 server startup이 durable unfinished `process_attempt`을 읽어 정리 주체가 되어야 한다. server control EOF를 관측할 keeper가 살아 있다고 가정하지 않는다.

이 네 항목은 platform spike와 OCR/Codex 설계 리뷰 대상이다. 해결 전에는 macOS/Linux containment 구현 task를 green으로 처리하지 않는다.

## Windows

- Job Object를 만들고 kill-on-close를 설정한다.
- payload primary thread는 suspended 상태로 생성한다.
- process를 Job에 assign한 뒤에만 resume한다.
- breakaway flag를 허용하지 않는다.
- assign/resume 실패는 process terminate+wait 뒤 spawn failure다.
- server crash로 마지막 Job handle이 닫히면 전체 associated tree가 종료된다.

## 공통 검증

| Scenario | macOS | Linux | Windows |
|---|---:|---:|---:|
| graceful direct child | required | required | required |
| signal-ignore force fallback | required | required | required |
| grandchild tree | required | required | required |
| leader exits before descendant | required | required | required |
| new process group/session escape | required | required | Job breakaway denied |
| double-fork, reparent, control FD closed | required | required | Job membership retained |
| env cleared before exec | required | required | Job membership retained |
| rapid exit before publish | required | required | required |
| cancel during adopt | required | required | required |
| server hard kill | required | required | required |
| keeper hard kill + startup recovery | required | required | N/A (Job owned by server) |
| identity-check/signal PID reuse race | safe handle required | safe handle required | process handle required |
| PID reuse/control process | required | required | required |
| stdout/stderr pressure | required | required | required |
| unreaped direct child | zero | zero | zero |

각 Unix escape case는 원 group 생존 수와 nonce+start identity live-set 수를 따로 기록한다. `group=0`만으로 통과하지 않는다. Windows는 Job accounting/active process count와 direct wait를 함께 기록한다.

실제 target job이 없는 결과를 다른 platform의 통과로 대체하지 않는다. 미지원 OS API가 확인되면 feature 완료가 아니라 명시적 blocker로 처리하고 spec scope 변경 리뷰를 거친다.
