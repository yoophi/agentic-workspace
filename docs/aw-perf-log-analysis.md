# AW 성능 로그(AW_PERF_LOG) 분석 가이드

agentic-workbench의 worktree session 성능 개선(specs/007) 검증에 쓰는 계측 로그의 활성화·수집·분석 방법을 정리한다. 검증 시나리오 자체는 `specs/007-worktree-session-performance/quickstart.md`, 판정 기준은 spec의 Success Criteria를 따른다.

## 1. 활성화와 수집

```bash
# dev 실행 (로그는 stderr로 출력된다)
AW_PERF_LOG=1 pnpm --filter agentic-workbench tauri dev

# 파일로 수집하려면
AW_PERF_LOG=1 pnpm --filter agentic-workbench tauri dev 2>&1 | tee /tmp/aw-perf.log
```

frontend 계측(`session:shell-rendered`, `session:graph-first-row`)은 dev 빌드에서 **앱 webview의 devtools 콘솔**에 `[perf] session:shell-rendered: 812ms` 형식으로 출력된다. 백엔드 로그와 채널이 다르니 둘 다 확인한다.

## 2. 로그 포맷

```text
perf kind=command name=<command명> wait_ms=<대기> run_ms=<실행>
perf kind=git     name=<git 작업명>  run_ms=<실행>
perf kind=watcher name=emit kind=<File|Git>
```

| 필드 | 의미 |
|---|---|
| `kind=command` | Tauri command 1회 실행. `wait_ms`는 invoke 도착→blocking pool 실행 시작까지 대기 시간, `run_ms`는 실행 시간 |
| `kind=git` | provider 내부 git subcommand 실행 시간. 현재 `worktree-status`(worktree별 `git status`)가 대상 |
| `kind=watcher` | 파일 watcher가 debounce 후 실제로 발행한 이벤트. `kind=File`(일반 파일) / `kind=Git`(commit 이력·브랜치 변화) |

로그 포맷은 디버깅용이며 외부 소비 계약이 아니다(specs/007 research R12).

## 3. 집계 스니펫

`$f`를 로그 파일 경로로 두고 사용한다.

```bash
# command별 호출 수 / 최대 실행·대기 시간
grep "^perf kind=command" "$f" | awk '{
  name=$3; sub("name=","",name);
  w=$4; sub("wait_ms=","",w); r=$5; sub("run_ms=","",r);
  c[name]++; if(r+0>mr[name])mr[name]=r+0; if(w+0>mw[name])mw[name]=w+0
} END{for(n in c) printf "%-30s n=%-4d max_run=%-6d max_wait=%d\n", n, c[n], mr[n], mw[n]}'

# watcher 이벤트 kind 분포 (idle 검증: Git이 0이어야 정상)
grep "^perf kind=watcher" "$f" | awk '{print $NF}' | sort | uniq -c

# worktree별 git status 비용 분포
grep "worktree-status" "$f" | awk '{
  r=$4; sub("run_ms=","",r); n++; if(r+0>m)m=r+0; s+=r
} END{printf "n=%d max=%dms avg=%dms\n", n, m, s/n}'

# graph 페이지별 실행 시간 추이 (뒤 페이지가 첫 페이지 2배 이내인지)
grep "name=get_worktree_git_graph" "$f" | awk '{r=$5; sub("run_ms=","",r); print NR": "r"ms"}'
```

## 4. 판정 기준 매핑

| 확인 항목 | 로그에서 보는 것 | 판정 |
|---|---|---|
| SC-001 세션 골격 표시 | webview 콘솔 `[perf] session:shell-rendered` | < 1,000ms |
| SC-002 직렬화 해소 | 모든 command의 `wait_ms` | 느린 command(예: status 포함 목록 수 초)가 실행 중일 때도 다른 command의 `wait_ms`가 0에 가까움 |
| SC-003 idle 되먹임 없음 | 세션을 연 채 10분간 `kind=watcher ... kind=Git` 발생 수 | 0회 (앱 내부의 주기적 `git status`가 이벤트를 유발하지 않음) |
| SC-005 선택 view만 조회 | graph view 진입 시 `list_worktree_git_history` 부재 | graph 진입 직후 로그에 history command가 없음 |
| SC-006 페이지 비용 | `get_worktree_git_graph`의 페이지별 `run_ms` | 뒤 페이지 ≤ 첫 페이지 × 2 |
| status lazy 동작 | 세션 refs 조회(`list_git_worktrees` 빠른 호출) 직후 `worktree-status` 로그 부재 | refs 조회는 status를 실행하지 않음 |

## 5. 해석 시 주의점

- **두 종류의 `list_git_worktrees`가 섞여 나온다.** 세션 route의 refs 조회(`includeStatus: false`, 수십 ms)와 프로젝트 상세/대시보드의 status 포함 조회(worktree당 `worktree-status` 로그 동반, 수 초 가능)는 같은 command 이름으로 찍힌다. 직전·직후의 `kind=git name=worktree-status` 동반 여부로 구분한다.
- **메인 창과 세션 창이 동시에 폴링한다.** 두 창이 열려 있으면 각자의 3초/30초 interval 로그가 섞인다. 특정 화면만 측정하려면 다른 창을 닫는다.
- **`kind=File` watcher 이벤트는 idle에서도 나올 수 있다.** dev 서버(HMR)나 외부 도구가 worktree 파일을 건드리면 정상 발행이다. SC-003의 기준은 `kind=Git`이다.
- **첫 실행 수치는 콜드 캐시다.** OS 파일 캐시가 비어 있으면 `worktree-status`가 수 배 느리다. 비교 측정은 같은 조건(2회째 이후)에서 한다.

## 6. 2026-07-02 S1 실측 요약

worktree 다수 프로젝트의 세션 화면을 dev 빌드로 연 첫 세션에서:

- 모든 command `wait_ms=0` — status 포함 목록 조회가 **14.9초** 걸리는 동안에도 refs 조회(21~74ms), branches(~30ms), 파일 읽기(0~1ms)가 즉시 응답. **직렬화 해소 확인(SC-002)**.
- 세션 route가 쓰는 refs 조회는 21~74ms. 개선 전 구조라면 세션 진입이 status 포함 목록(이 환경에서 최대 14.9초)에 막혔을 것이다.
- graph 진입 시 `get_worktree_git_graph` 1회(300ms), `list_worktree_git_history` 0회 — **선택 view만 조회 확인(SC-005)**.
- watcher 이벤트는 `kind=File` 10회, **`kind=Git` 0회** — 주기적 status가 계속 도는 중에도 되먹임 없음(SC-003 예비 신호, 10분 idle 관찰로 확정 필요).
- **후속 개선 후보 실측 확인**: 메인 창(프로젝트 상세/대시보드)의 3초 interval status 폴링이 사이클당 worktree 수 × 평균 739ms(최대 7.8초)의 git 작업을 계속 발생시킨다. 조사 문서의 P2 항목("3초 interval refresh")이 큰 비용임이 확인되었으므로 interval 완화 또는 focus 기반 refresh를 후속 작업으로 권장한다.
