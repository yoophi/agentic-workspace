#!/bin/bash
# 사용: run-probe.sh <run-name> <dev|release> <identifier> <scenario: default|refresh|quit> [hold]
# hold가 있으면 probe 결과 뒤 앱을 끝내지 않고 PID를 $R/app-pids.txt에 남긴다(quit 스모크용).
. "$(dirname "$0")/lib.sh"
NAME=${1:-}; ORIGIN=${2:-}; IDENT=${3:-}; SCEN=${4:-}; HOLD=${5:-}
need "$NAME" name; need "$ORIGIN" origin; need "$IDENT" identifier; need "$SCEN" scenario
R="$SMOKE/$NAME"
[ -e "$R" ] && { echo "run dir exists: $R (use a new name)"; exit 8; }
mkdir -p "$R/work"
DATA="$HOME/Library/Application Support/$IDENT"
echo "data=$DATA" > "$R/meta.txt"
[ -e "$DATA/workbench/server/server.json" ] && { echo "server.json already exists for $IDENT"; exit 7; }
AGENT="python3 $WT/crates/workbench-core/tests/support/agents/fake_acp_permission_agent.py --echo --log $R/agent.log"
CWD=$(cd "$R/work" && pwd -P)
export AW_APP_TRANSPORT_PROBE_FILE="$R/probe.json" AW_APP_PROBE_AGENT_COMMAND="$AGENT" AW_APP_PROBE_CWD="$CWD"
# NEIGHBOR=1이면 경로를 주지 않아 "앱 실행 파일 옆" 탐색을 쓴다(배포 출처).
if [ "${NEIGHBOR:-}" = 1 ]; then unset AW_WORKBENCH_SERVER_PATH; echo "server-path=neighbor" >> "$R/meta.txt"; else export AW_WORKBENCH_SERVER_PATH="$SERVER_BIN"; fi
[ "$SCEN" = default ] || export AW_APP_PROBE_SCENARIO="$SCEN"
unset AW_WORKBENCH_MODE
cd "$WT"
if [ "$ORIGIN" = dev ]; then
  CARGO_INCREMENTAL=0 VITE_AW_DEBUG_PROBE=1 nohup pnpm --filter @yoophi/agentic-workbench tauri dev --config "{\"identifier\":\"$IDENT\"}" > "$R/app.log" 2>&1 &
else
  nohup "$WT/target/debug/agentic-workbench" > "$R/app.log" 2>&1 &
fi
LAUNCH=$!
echo "launch=$LAUNCH" >> "$R/meta.txt"
deadline=$((SECONDS+420)); st=timeout
while [ $SECONDS -lt $deadline ]; do
  if ! kill -0 "$LAUNCH" 2>/dev/null; then st=app-exited; break; fi
  if grep -q "error\[E\|panicked\|ELIFECYCLE" "$R/app.log"; then st=app-failed; break; fi
  if [ -s "$R/probe.json" ]; then
    python3 - "$R/probe.json" "$SCEN" <<'EOF'
import json,sys
d=json.load(open(sys.argv[1])); s=sys.argv[2]
if s=='refresh': sys.exit(0 if d.get('phase')==2 or d.get('result')=='error' else 1)
sys.exit(0 if 'result' in d else 1)
EOF
    [ $? -eq 0 ] && { st=done; break; }
  fi
  sleep 2
done
echo "probe-status=$st" | tee -a "$R/meta.txt"
SPID=$(server_pid "$DATA"); echo "server-pid=$SPID" >> "$R/meta.txt"
# 서버(와 그 자식 agent)는 앱 체인에서 뺀다 — 앱 종료와 서버 생존을 따로 본다.
SERVER_TREE=" ${SPID:-none} $( [ -n "$SPID" ] && descendants "$SPID" | tr '\n' ' ') "
APP_PIDS=""
for p in $LAUNCH $(descendants "$LAUNCH"); do
  case "$SERVER_TREE" in *" $p "*) ;; *) APP_PIDS="$APP_PIDS $p" ;; esac
done
echo "app-pids=$APP_PIDS" >> "$R/meta.txt"
echo "server-tree=$SERVER_TREE" >> "$R/meta.txt"
if [ -n "$SPID" ] && is_our_server "$SPID" "$DATA"; then echo "server-verified=yes" >> "$R/meta.txt"; else echo "server-verified=no" >> "$R/meta.txt"; fi
if [ -z "$HOLD" ]; then
  # 앱 체인만 끝낸다. 서버는 호출자가 확인 뒤 정확한 PID로 끝낸다.
  kill_exact "$R/kills.txt" $APP_PIDS
fi
cat "$R/meta.txt"
