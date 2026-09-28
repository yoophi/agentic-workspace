# 045 platform feasibility evidence

## 판정

production consumer migration prerequisite는 **PENDING**이다. 아래 macOS spike는 environment inventory/parser 가정 일부를 확인했지만, env-clear descendant discovery와 identity-check/signal 원자성을 해결하지 못했다. Linux와 Windows의 실제 target 실행 결과도 아직 없다. 이 상태는 045 전체 목표를 포기하거나 scope를 줄인다는 뜻이 아니며, 대안 API/권한 spike를 계속하기 위한 gate다.

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
- `python3 -m unittest scripts/tests/test_process_spawn_inventory.py`: 3 passed, exit 0
- negative fixtures는 aliased `Command`, 동일 파일의 추가 constructor, `#[cfg(test)]` 뒤 production constructor를 각각 검출한다.
- 현재 ServerOwned direct spawn은 baseline으로 별도 출력되며, 최종 migration gate에서는 `--enforce-supervised`로 0을 요구한다.

## Linux / Windows

| Target | Actual executed tests | Result | Evidence status |
|---|---:|---|---|
| Linux x86_64 | 0 | 미실행 | 없음 |
| Windows x86_64 | 0 | macOS run에서 cfg로 제외 | 없음 |

`windows_job_feasibility.rs`의 bool invariant test는 Job API 실행 증거가 아니다. Windows evidence는 suspended `CreateProcessW`, assign-before-resume, breakaway denial, kill-on-close, Job accounting와 wait를 실제 Windows job에서 실행한 뒤에만 기록한다.

현재 branch에는 `045-process-supervisor` push에서 macOS/Linux/Windows actual-target job을 실행하는 feasibility matrix가 있다. 로컬 cross-target strict Clippy는 Linux와 Windows 모두 exit 0이지만 실행 증거로 세지 않는다. Linux의 cgroup probe는 unified hierarchy, `cgroup.kill`, 현재 cgroup 아래 directory 생성 권한만 관측하며, directory 생성 성공만으로 env-clear descendant containment를 입증하지 않는다. 실제 run ID/HEAD/test 수/exit code는 branch CI가 끝난 뒤 이 표에 기록한다.

## 다음 feasibility 작업

1. macOS `proc_terminate`, Mach task/audit-token 계열 API가 ordinary signed app 권한에서 identity-safe signal 또는 descendant tracking을 제공하는지 격리 확인한다.
2. 제공하지 못하면 Endpoint Security/system extension 같은 entitlement 요구 대안과 제품 배포 가능성을 별도 설계 입력으로 기록한다.
3. Linux는 pidfd와 cgroup v2 delegation을 분리해 확인한다. pidfd가 known PID reuse만 막고 env-clear descendant discovery는 해결하지 못하는지 실제 fixture로 판정한다.
4. Windows는 실제 Job Object API spike와 CI target 실행을 추가한다.
