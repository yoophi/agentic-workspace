# 045 platform feasibility evidence

## 판정

production consumer migration prerequisite는 **PENDING**이다. actual-target API probes는 세 OS에서 실행됐지만 macOS/Linux required containment와 Windows server-crash 경로는 아직 입증되지 않았다. 이 상태는 045 전체 목표를 포기하거나 scope를 줄인다는 뜻이 아니며, release-shaped 대안 spike를 계속하기 위한 gate다.

## 실행 환경

- 실행일: 2026-09-28
- host: macOS 15.6.1 (24G90), Apple Silicon arm64
- Rust: 1.98.1
- Xcode: 26.3 (17C529)
- SDK: macOS 26.2
- reviewed tree 시작점: `1cb271d`

## macOS isolated spike

실행 명령:

```text
cargo clippy -p process-supervisor --all-targets -- -D warnings
cargo test -p process-supervisor --test platform_feasibility -- --nocapture
```

결과:

- Clippy: exit 0
- `platform_feasibility`: 10 passed, 0 failed, 0 ignored, 0 filtered, exit 0
- parser unit: empty argv를 포함한 synthetic `KERN_PROCARGS2` payload에서 nonce environment 추출 통과
- fixture self-check: child 자체에서 injected marker 존재를 1 bit로 확인
- same-PID sysctl: returned size 14,332 bytes, errno 없음, raw marker 존재, parsed key 존재, parsed value 일치
- env-clear + new-session escaped child: returned size 620 bytes, errno 없음, raw marker 없음, parsed key 없음
- parent→arbitrary child audit-token acquisition: `task_for_pid` status 5, token/signal 단계 미도달
- cooperative direct child audit-token handshake: self token 전달 뒤 `proc_signal_with_audittoken` 성공, exact child 2초 안에 종료
- keeper-only hard kill: keeper 종료 뒤 env-clear/new-session descendant가 같은 start identity로 계속 생존하고 nonce는 보이지 않음
- server+keeper hard kill: 두 owner 종료 뒤 descendant가 계속 생존해 startup recovery anchor가 필요함을 재현

앞선 `/bin/sleep` 34-byte/marker-miss 관측은 권한 제한 근거에서 제외했다. self-check, raw bytes와 corrected parser를 함께 검사한 결과 현재 배포 권한에서는 same-UID fixture environment를 읽을 수 있었다.

### 아직 성립하지 않은 요구

- env를 지우고 새 session으로 이동한 descendant는 nonce inventory에서 사라진다.
- SDK header scan에서 `proc_pidinfo`, PID 기반 `proc_terminate`, `EVFILT_PROC`, audit-token signal을 확인했다. 협력 direct child는 token handshake로 identity-safe signal이 가능하지만 ordinary parent가 임의 descendant token을 얻는 `task_for_pid`는 실제 권한에서 거절됐다.
- macOS SDK는 `NOTE_TRACK/NOTE_CHILD`가 10.5 이후 지원되지 않는다고 명시한다. 따라서 kqueue fork-tree 추적을 근거로 쓸 수 없다.
- known PID의 start identity를 확인한 뒤 PID signal을 보내는 사이의 reuse window는 닫히지 않았다.
- Endpoint Security header는 fork/exec event와 entitlement 오류를 정의하지만 SDK/runtime에서 link 가능한 `EndpointSecurity` framework를 찾지 못해 probe compile이 실패했다. entitlement 적합성 이전에 현재 빌드/배포 artifact 경로부터 미해결이다.

`EscapedPidCleanup`은 시험 잔여물 정리용이다. fixture가 기록한 최초 `ProcessStartIdentity`와 signal 직전 identity 전체를 비교하지만, 비교와 `kill(pid)`는 원자적이지 않다. 이 guard의 성공을 production feasibility 근거로 세지 않는다.

## inventory gate evidence

- `python3 scripts/check-process-spawn-inventory.py --json`: unknown 0, stale 0, undocumented 0, direct-count drift 0
- `python3 -m unittest scripts/tests/test_process_spawn_inventory.py`: 현재 HEAD 5 passed, exit 0
- negative fixtures는 aliased `Command`, 동일 파일의 추가 constructor, cfg(test) module/import 뒤 production constructor와 comment/string delimiter 우회를 검출한다.
- 현재 ServerOwned direct spawn은 baseline으로 별도 출력되며, 최종 migration gate에서는 `--enforce-supervised`로 0을 요구한다.

## Actual-target CI

- run: `36429844170`
- exact HEAD: `c3e292f91a39d522e38bfb1449c2d28c2d767324`
- macOS job `108953038337`: strict Clippy exit 0; parser 1, platform feasibility 10, inventory 2 passed; 각 suite filtered 0; job exit 0
- Linux job `108953038084`: strict Clippy exit 0; platform feasibility 9, inventory 2 passed; 각 suite filtered 0; job exit 0
- Windows job `108953037849`: strict Clippy exit 0; inventory 2, Windows feasibility 2 passed; 각 suite filtered 0; job exit 0

0-test fixture binary harness는 compile 확인일 뿐 위 passed 수에 합산하지 않는다. 이 matrix 성공은 spike가 관측과 실패 경계를 예상대로 기록했다는 의미이며 세 target containment가 모두 성립했다는 뜻이 아니다. 전체 `validate` job은 이 문서 갱신 시점에 진행 중이므로 전체 gate 근거로 사용하지 않는다.

## Linux

- same-UID `/proc/<pid>/environ` marker 관측과 env-clear 후 marker 소실을 실제 runner에서 확인했다.
- `pidfd_open`으로 exact known child handle을 확보하고 `pidfd_send_signal` 뒤 2초 안에 wait 및 원 start identity 소실을 확인했다.
- unified cgroup v2와 `cgroup.kill`은 존재했다. 현재 `/system.slice/hosted-compute-agent.service` 아래 child cgroup 생성은 `EACCES(13)`로 거절됐다.
- 따라서 pidfd는 이미 발견한 PID 재사용만 막고 env-clear+reparent descendant를 발견하지 못한다. cgroup directory 관측도 containment 증명이 아니다. `env_clear_descendant_trackable=false`이며 prerequisite는 실패다.

## Windows

| Target | Actual executed tests | Result | Evidence status |
|---|---:|---|---|
| Linux x86_64 | 11 | job 108953038084 exit 0 | API 관측 성공, required containment 실패 |
| Windows x86_64 | 4 | job 108953037849 exit 0 | Job API probe PASS; platform prerequisite PENDING |

Windows 실제 API 시험은 suspended create, assign-before-resume, primary thread resume 1회, active processes 2를 관측했다. explicit breakaway는 `ERROR_ACCESS_DENIED`였고 명시적 `drop(job)` 뒤 미리 확보한 direct/descendant process handle 양쪽이 bounded wait 안에 종료됐다. 별도 owner/server hard-kill fixture는 아직 없다. 2건 중 1건은 이 실제 API 시험이고 1건은 bool invariant이므로 서로 구분한다.

현재 branch에는 `045-process-supervisor` push에서 actual-target matrix를 실행하는 임시 trigger가 있다. 정식 PR 전 feasibility 증거를 얻기 위한 것이며 최종 workflow 정책은 구현 리뷰에서 다시 판정한다.

## 다음 feasibility 작업

1. macOS 15 release-shaped 표준 Endpoint Security system extension의 entitlement/signing/TCC/event/crash recovery를 검증한다.
2. Linux installed systemd unit의 delegation과 birth-time cgroup placement, direct-launch fail-closed와 crash recovery를 검증한다.
3. PID 재사용 경합을 결정적으로 유발해 safe handle과 unsafe PID check→signal을 대조한다.
4. Windows owner/server hard-kill과 전체 containment matrix를 실제 target에서 검증한다.
