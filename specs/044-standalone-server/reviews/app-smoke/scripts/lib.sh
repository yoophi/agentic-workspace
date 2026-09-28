# 044 앱 스모크 공용 함수. 삭제 명령 없음. 종료는 이 실행이 띄운 정확한 PID만.
set -u
WT=/Users/yoophi/project/worktrees/044-standalone-server
SMOKE=<scratchpad>/044/smoke
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
kill_exact() {
  local log=$1; shift
  local dir p now
  dir=$(dirname "$log")
  for p in "$@"; do
    need "$p" pid
    if [ ! -s "$dir/pids/$p" ]; then echo "skip $p: identity was not recorded" >> "$log"; continue; fi
    now=$(proc_identity "$p")
    if [ -z "$now" ]; then echo "skip $p: already gone" >> "$log"; continue; fi
    if [ "$now" != "$(cat "$dir/pids/$p")" ]; then echo "skip $p: identity changed (pid reused?)" >> "$log"; continue; fi
    echo "kill $p: $(echo "$now" | cut -c1-200)" >> "$log"
    kill "$p" 2>/dev/null
  done
}
