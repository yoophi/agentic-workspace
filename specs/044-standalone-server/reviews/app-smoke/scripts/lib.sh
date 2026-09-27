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

# 정확한 PID 목록만 TERM으로 끝낸다. 목록과 결과를 기록한다.
kill_exact() {
  local log=$1; shift
  local p
  for p in "$@"; do
    need "$p" pid
    echo "kill $p: $(ps -o command= -p "$p" 2>/dev/null | cut -c1-160)" >> "$log"
    kill "$p" 2>/dev/null
  done
}
