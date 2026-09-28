# 044 앱 스모크 공용 함수. 삭제 명령 없음. 종료는 이 실행이 띄운 정확한 PID만.
set -u
WT=/Users/yoophi/project/worktrees/044-standalone-server
# 스모크 작업 디렉터리(실행 결과·빌드한 도우미). 저장소 사본은 환경 변수로 받는다(세션 scratchpad 경로를 커밋하지 않음).
SMOKE=${AW_SMOKE_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}
SERVER_BIN="$WT/target/debug/agentic-workbench-server"

need() { [ -n "${1:-}" ] || { echo "empty variable: $2" >&2; exit 9; }; }

# 한 PID의 자손(재귀) PID를 출력한다.
descendants() {
  local p
  for p in $(pgrep -P "$1" 2>/dev/null); do echo "$p"; descendants "$p"; done
}

# 데이터 디렉터리의 server.json에서 pid를 읽는다(없으면 빈 문자열).
server_pid() {
  python3 -c "import json,sys;print(json.load(open(sys.argv[1])).get('pid',''))" "$1/workbench/server/server.json" 2>/dev/null
}

# 서버 PID가 이 데이터 디렉터리의 우리 서버 실행 파일인지 확인한다(명령줄에 실행 파일 경로와 데이터 디렉터리).
is_our_server() {
  local pid=$1 data=$2 cmd
  cmd=$(ps -o command= -p "$pid" 2>/dev/null) || return 1
  case "$cmd" in *"$SERVER_BIN"*) ;; *) return 1 ;; esac
  case "$cmd" in *"$data"*) return 0 ;; *) return 1 ;; esac
}

# 프로세스 신원: 시작 시각 + 명령줄. PID는 재사용될 수 있으므로 PID만으로 같은 프로세스라고 보지 않는다.
proc_identity() {
  ps -o lstart= -o command= -p "$1" 2>/dev/null
}

# 발견한 PID의 신원을 그 실행 디렉터리의 `pids/`에 기록한다. `kill_exact`는 기록된 신원과 지금 신원이 같을 때만 신호를 보낸다.
remember() {
  local dir=$1; shift
  local p
  mkdir -p "$dir/pids"
  for p in "$@"; do
    need "$p" pid
    proc_identity "$p" > "$dir/pids/$p"
  done
}

# 정확한 PID만 TERM으로 끝낸다(신호 직전에 신원을 다시 확인). 기록이 없거나, 이미 없거나, 신원이 바뀌었으면 보내지 않는다.
# 모든 pid에 신호를 보냈으면 0, 하나라도 건너뛰었으면 1.
kill_exact() {
  local log=$1; shift
  local dir p now rc=0
  dir=$(dirname "$log")
  for p in "$@"; do
    need "$p" pid
    if [ ! -s "$dir/pids/$p" ]; then echo "skip $p: identity was not recorded" >> "$log"; rc=1; continue; fi
    now=$(proc_identity "$p")
    if [ -z "$now" ]; then echo "skip $p: already gone" >> "$log"; rc=1; continue; fi
    if [ "$now" != "$(cat "$dir/pids/$p")" ]; then echo "skip $p: identity changed (pid reused?)" >> "$log"; rc=1; continue; fi
    echo "kill $p: $(echo "$now" | cut -c1-200)" >> "$log"
    kill "$p" 2>/dev/null || rc=1
  done
  return $rc
}

# 앞 프로세스가 정확히 그 pid인가(전역 키 입력은 앞 프로세스로 간다).
front_is() {
  [ "$(osascript -e 'tell application "System Events" to get unix id of first process whose frontmost is true' 2>/dev/null)" = "$1" ]
}

# 대상 앱을 앞으로 가져오고 앞 프로세스가 그 pid가 될 때까지 조건 대기(상한 10초). 성공하면 0.
wait_front() {
  local apid=$1 bid=$2 i
  for i in $(seq 1 40); do
    osascript -e "tell application id \"$bid\" to activate" >/dev/null 2>&1
    front_is "$apid" && return 0
    sleep 0.25
  done
  return 1
}

# 전역 키 입력: 앞 프로세스가 그 pid임을 두 번(대기 뒤·보내기 직전) 확인한 뒤에만 보낸다. 아니면 보내지 않고 1.
send_key_to() {
  local apid=$1 bid=$2 key=$3
  wait_front "$apid" "$bid" || return 1
  front_is "$apid" || return 1
  osascript -e "tell application \"System Events\" to keystroke \"$key\" using command down" >/dev/null 2>&1
}

# quit 경로 (g): 기록한 신원이 같을 때만 SIGTERM. 보냈으면 0, 보내지 않았으면 1(`kill_exact`와 같음).
send_sigterm() {
  kill_exact "$1" "$2"
}

# quit 경로 판정: <sent yes|no> <gone yes|no> <path>. 종료 동작을 보냈고 그 뒤 PID가 사라졌을 때만 `path-exercised=yes`(0).
# 아니면 `path-exercised=no`와 그 이유를 적고 5(무효).
quit_verdict() {
  local sent=$1 gone=$2 path=$3
  if [ "$sent" = yes ] && [ "$gone" = yes ]; then echo "path-exercised=yes"; return 0; fi
  if [ "$sent" != yes ]; then
    echo "path-exercised=no (the quit action for path $path was not sent; app-gone=$gone is not evidence of this path)"
  else
    echo "path-exercised=no (the app is still running after path $path)"
  fi
  return 5
}

# quit 스모크 최종 결과: <경로 판정 종료 코드> <owner-check 종료 코드>. 경로 무효(0 아님)면 그 코드(5), 경로는 유효한데 run 지속
# 확인이 실패하면 6, 둘 다 통과하면 0. 한 줄 `smoke-result=`을 적는다.
smoke_final() {
  local verdict=$1 oc=$2 tok=${3:-skip}
  if [ "$verdict" != 0 ]; then echo "smoke-result=invalid-path (exit $verdict)"; return "$verdict"; fi
  if [ "$oc" != 0 ]; then echo "smoke-result=owner-check-failed (owner-check exit $oc; exit 6)"; return 6; fi
  if [ "$tok" != skip ] && [ "$tok" != 0 ]; then echo "smoke-result=token-check-failed (exit 8)"; return 8; fi
  echo "smoke-result=ok"; return 0
}

# 토큰 검사 판정(Codex r9 docs): <기대 상태> <token-check.py 출력>. 출력의 `status`가 기대와 같으면 0, 다르거나 검사 오류(출력
# 없음·해석 불가)면 8. 한 줄 `token-result=`을 적는다. 토큰 값은 출력에 없다(token-check.py가 상태 코드만 낸다).
token_verdict() {
  local want=$1 got
  got=$(printf '%s' "$2" | python3 -c 'import json,sys
try: print(json.load(sys.stdin).get("status", "error"))
except Exception: print("error")' 2>/dev/null)
  [ -n "$got" ] || got=error
  if [ "$got" = "$want" ]; then echo "token-result=ok (status $got, expected $want)"; return 0; fi
  echo "token-result=failed (status $got, expected $want)"; return 8
}

# 창 닫기 스모크 최종 결과(Codex r9 docs): <run 제거 yes|no> <토큰 판정 코드|skip>. run이 대기 상한 뒤에도 남으면 7, 토큰이
# 폐기되지 않았거나 검사 오류면 8, 둘 다 통과하면 0. 한 줄 `close-result=`을 적는다.
close_final() {
  local removed=$1 tok=${2:-skip} win=${3:-0} want=${4:-yes}
  if [ "$win" = 6 ]; then echo "close-result=invalid-attempt (the close action failed; exit 6)"; return 6; fi
  if [ "$win" != 0 ]; then echo "close-result=unexpected-window-state (exit 9)"; return 9; fi
  if [ "$removed" != "$want" ]; then
    if [ "$want" = yes ]; then echo "close-result=run-not-removed (exit 7)"; else echo "close-result=run-removed-unexpectedly (exit 7)"; fi
    return 7
  fi
  if [ "$tok" != skip ] && [ "$tok" != 0 ]; then echo "close-result=token-not-revoked (exit 8)"; return 8; fi
  echo "close-result=ok"; return 0
}

# 창 닫기 경로의 창 상태 판정(Codex r10·r11 docs): <경로 a|b1|b2|f> <닫기 동작 종료 코드> <앱 생존 yes|no> <남은 창 목록>.
# 동작 실패는 무효(6). 닫는 창은 경로마다 다르다: (a)(b1)은 메인 창을 닫으므로 남은 창이 정확히 "Settings", (b2)는 앞에 있는
# Settings에 Cmd+W를 보내므로 남은 창이 정확히 "Agentic Workbench"(메인 창과 run 유지), (f)는 마지막 창이라 앱이 끝나야 한다.
# 어긋나면 9. 한 줄 `window-result=`을 적는다.
window_verdict() {
  local path=$1 act=$2 alive=$3 windows=$4
  if [ "$act" != 0 ]; then echo "window-result=action-failed (exit $act; attempt invalid)"; return 6; fi
  case "$path" in
    f) if [ "$alive" = no ]; then echo "window-result=ok (last window closed, app exited)"; return 0; fi
       echo "window-result=unexpected (app still running after closing the last window)"; return 9 ;;
    b2) if [ "$alive" = yes ] && [ "$windows" = "Agentic Workbench" ]; then echo "window-result=ok (front Settings closed, main window kept)"; return 0; fi
        echo "window-result=unexpected (app alive=$alive, windows=[$windows]; expected only the main window to remain)"; return 9 ;;
    *) if [ "$alive" = yes ] && [ "$windows" = Settings ]; then echo "window-result=ok (main window closed, Settings kept)"; return 0; fi
       echo "window-result=unexpected (app alive=$alive, windows=[$windows]; expected only Settings to remain)"; return 9 ;;
  esac
}
