#!/bin/bash
# 사용: quit-run.sh <run-name> <app-bundle-path> <bundle-id> <product-name> <path: c|d|e|g>
# run 시작(probe quit) → 그 경로로 앱 종료 → PID 소멸 → owner-check.py(진행·출력·취소) → 서버를 정확한 PID로 정지.
. "$(dirname "$0")/lib.sh"
NAME=${1:-}; APP=${2:-}; BID=${3:-}; PRODUCT=${4:-}; QUIT=${5:-}
need "$NAME" name; need "$APP" app; need "$BID" bid; need "$PRODUCT" product; need "$QUIT" path
R="$SMOKE/$NAME"
[ -e "$R" ] && { echo "run dir exists: $R"; exit 8; }
mkdir -p "$R/work"
DATA="$HOME/Library/Application Support/$BID"
[ -e "$DATA/workbench/server/server.json" ] && { echo "server.json already exists for $BID"; exit 7; }
APP_EXE="$APP/Contents/MacOS/agentic-workbench"
SRV_EXE="$APP/Contents/MacOS/agentic-workbench-server"
app_pids() { ps -axo pid=,command= | awk -v exe="$APP_EXE" '{pid=$1; $1=""; sub(/^ /,""); if ($0==exe || index($0, exe" ")==1) print pid}'; }
log() { echo "$*" | tee -a "$R/meta.txt"; }
BEFORE=" $(app_pids | tr '\n' ' ') "
AGENT="python3 $WT/crates/workbench-core/tests/support/agents/fake_acp_permission_agent.py --echo --log $R/agent.log"
# BUSY=1(T045 진행 중 turn): agent가 시작 turn을 문 파일로 붙잡고, probe는 완료 전에 보고한다. 종료 뒤 owner-check가 문을 푼다.
SCEN=quit; PHASE=ready-to-quit
if [ "${BUSY:-}" = 1 ]; then AGENT="$AGENT --end-turn-gate $R/turn.gate --after-gate-chunk"; SCEN=quit-busy; PHASE=ready-to-quit-busy; fi
# TOKEN=1(Codex 코드 리뷰): close-token probe가 이 창 토큰을 비밀 파일(0600)에 넘긴다. 종료 뒤 같은 토큰이 거절되는지 본다.
if [ "${TOKEN:-}" = 1 ]; then SCEN=close-token; PHASE=ready-to-close; fi
CWD=$(cd "$R/work" && pwd -P)
open -n "$APP" --env AW_APP_TRANSPORT_PROBE_FILE="$R/probe.json" --env AW_APP_PROBE_SCENARIO=$SCEN --env AW_APP_PROBE_SECRET_FILE="$R/secret.json" \
  --env AW_APP_PROBE_AGENT_COMMAND="$AGENT" --env AW_APP_PROBE_CWD="$CWD" --stdout "$R/app.log" --stderr "$R/app.log"
APID=""
for i in $(seq 1 60); do
  for p in $(app_pids); do case "$BEFORE" in *" $p "*) ;; *) APID=$p ;; esac; done
  [ -n "$APID" ] && break; sleep 0.5
done
need "$APID" app-pid
remember "$R" "$APID"
log "app-pid=$APID quit-path=$QUIT"
deadline=$((SECONDS+240)); st=timeout
while [ $SECONDS -lt $deadline ]; do
  kill -0 "$APID" 2>/dev/null || { st=app-exited; break; }
  if [ -s "$R/probe.json" ] && python3 -c "import json,sys; sys.exit(0 if 'result' in json.load(open(sys.argv[1])) else 1)" "$R/probe.json" 2>/dev/null; then st=done; break; fi
  sleep 1
done
log "probe-status=$st"
RUN=$(python3 -c "import json,sys;d=json.load(open(sys.argv[1]));print(d.get('runId','') if d.get('result')=='ok' and d.get('phase')==sys.argv[2] else '')" "$R/probe.json" "$PHASE" 2>/dev/null)
SPID=$(server_pid "$DATA")
log "run-id=${RUN:-none} server-pid=${SPID:-none}"
SCMD=$(ps -o command= -p "${SPID:-0}" 2>/dev/null)
case "$SCMD" in "$SRV_EXE serve --data-dir $DATA"*) log "server-verified=yes"; remember "$R" "$SPID" ;; *) log "server-verified=no cmd=$SCMD" ;; esac
if [ -z "$RUN" ] || [ -z "$SPID" ]; then log "abort: no run or server"; kill_exact "$R/kills.txt" "$APID"; exit 3; fi
[ "${TOKEN:-}" = 1 ] && log "token-before-quit=$(python3 "$SMOKE/token-check.py" "$R/secret.json")"
SERVER_LINES_BEFORE=$(wc -l < "$DATA/workbench/server/server.log")
case "$QUIT" in
  c) osascript -e "tell application id \"$BID\" to activate" >/dev/null 2>&1; sleep 1
     osascript -e 'tell application "System Events" to keystroke "q" using command down' >> "$R/quit.txt" 2>&1 ;;
  d) osascript -e "tell application \"System Events\" to tell process \"Dock\" to tell UI element \"$PRODUCT\" of list 1 to perform action \"AXShowMenu\"" >> "$R/quit.txt" 2>&1
     # 메뉴가 열릴 때까지 조건 대기(상한 10초) 뒤 누른다. 열리지 않으면 경로를 시험하지 못한 것으로 기록한다.
     menu=no; for i in $(seq 1 40); do
       [ "$(osascript -e "tell application \"System Events\" to tell process \"Dock\" to exists menu 1 of UI element \"$PRODUCT\" of list 1" 2>/dev/null)" = true ] && { menu=yes; break; }; sleep 0.25
     done; log "dock-menu-open=$menu"
     osascript -e "tell application \"System Events\" to tell process \"Dock\" to tell UI element \"$PRODUCT\" of list 1 to click menu item \"Quit\" of menu 1" >> "$R/quit.txt" 2>&1 ;;
  e) osascript -e "tell application id \"$BID\" to quit" >> "$R/quit.txt" 2>&1 ;;
  g) kill -TERM "$APID" ;;
  *) log "unknown path"; exit 4 ;;
esac
gone=no
for i in $(seq 1 60); do kill -0 "$APID" 2>/dev/null || { gone=yes; break; }; sleep 0.5; done
log "app-gone=$gone"
if [ "$gone" != yes ]; then log "app still running after quit path; stopping exact pid"; log "path-exercised=no (this run is invalid evidence for path $QUIT)"; kill_exact "$R/kills.txt" "$APID"; else log "path-exercised=yes"; fi
log "server-alive-after-quit=$(kill -0 "$SPID" 2>/dev/null && echo yes || echo no)"
[ "${TOKEN:-}" = 1 ] && log "token-after-quit=$(python3 "$SMOKE/token-check.py" "$R/secret.json")"
python3 "$SMOKE/status.py" "$DATA" > "$R/status-after-quit.json" 2>&1; log "status-after-quit=$(cat "$R/status-after-quit.json")"
tail -n +"$((SERVER_LINES_BEFORE+1))" "$DATA/workbench/server/server.log" > "$R/server-after-quit.log"
if [ "${BUSY:-}" = 1 ]; then
  case "$(cat "$R/status-after-quit.json")" in *'"busyRuns": 0'*|*'"busyRuns": null'*) log "busy-after-quit=no (the turn is not in flight)" ;; *) log "busy-after-quit=yes" ;; esac
  [ -e "$R/turn.gate" ] && log "gate-exists-before-observe=yes" || log "gate-exists-before-observe=no"
  python3 "$WT/specs/044-standalone-server/reviews/app-smoke/owner-check.py" --data-dir "$DATA" --run-id "$RUN" --observe-turn --release-file "$R/turn.gate" > "$R/owner-check.json" 2> "$R/owner-check.err"
else
python3 "$WT/specs/044-standalone-server/reviews/app-smoke/owner-check.py" --data-dir "$DATA" --run-id "$RUN" > "$R/owner-check.json" 2> "$R/owner-check.err"
fi
log "owner-check-exit=$?"
cat "$R/owner-check.json"
# 서버를 정확한 PID로 정지(확인된 경우만).
case "$SCMD" in "$SRV_EXE serve --data-dir $DATA"*) kill_exact "$R/kills.txt" "$SPID" ;; esac
for i in $(seq 1 60); do kill -0 "$SPID" 2>/dev/null || break; sleep 0.5; done
log "server-stopped=$(kill -0 "$SPID" 2>/dev/null && echo no || echo yes)"
