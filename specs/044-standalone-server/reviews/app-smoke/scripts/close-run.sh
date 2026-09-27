#!/bin/bash
# 사용: close-run.sh <run-name> <app-bundle> <bundle-id> <path: a|b1|b2|f>
# run 시작(probe quit 시나리오, run을 남김) → 창 닫기 경로 → 앱 상태·창 목록 → bench.list에서 run 사라짐 확인 → 정리(정확한 PID).
. "$(dirname "$0")/lib.sh"
NAME=${1:-}; APP=${2:-}; BID=${3:-}; CLOSE=${4:-}
need "$NAME" name; need "$APP" app; need "$BID" bid; need "$CLOSE" path
R="$SMOKE/$NAME"; [ -e "$R" ] && { echo "run dir exists"; exit 8; }; mkdir -p "$R/work"
DATA="$HOME/Library/Application Support/$BID"
[ -e "$DATA/workbench/server/server.json" ] && { echo "server.json exists"; exit 7; }
APP_EXE="$APP/Contents/MacOS/agentic-workbench"; SRV_EXE="$APP/Contents/MacOS/agentic-workbench-server"
app_pids() { ps -axo pid=,command= | awk -v exe="$APP_EXE" '{pid=$1; $1=""; sub(/^ /,""); if ($0==exe || index($0, exe" ")==1) print pid}'; }
log() { echo "$*" | tee -a "$R/meta.txt"; }
sx() { osascript -e "tell application \"System Events\" to tell (first process whose unix id is $APID) to $1" 2>&1; }
BEFORE=" $(app_pids | tr '\n' ' ') "
AGENT="python3 $WT/crates/workbench-core/tests/support/agents/fake_acp_permission_agent.py --echo --log $R/agent.log"
CWD=$(cd "$R/work" && pwd -P)
SCEN=quit; [ "${CLOSE_TOKEN:-}" = 1 ] && SCEN=close-token
open -n "$APP" --env AW_APP_TRANSPORT_PROBE_FILE="$R/probe.json" --env AW_APP_PROBE_SCENARIO=$SCEN --env AW_APP_PROBE_SECRET_FILE="$R/secret.json" \
  --env AW_APP_PROBE_AGENT_COMMAND="$AGENT" --env AW_APP_PROBE_CWD="$CWD" --stdout "$R/app.log" --stderr "$R/app.log"
APID=""
for i in $(seq 1 60); do for p in $(app_pids); do case "$BEFORE" in *" $p "*) ;; *) APID=$p ;; esac; done; [ -n "$APID" ] && break; sleep 0.5; done
need "$APID" app-pid; log "app-pid=$APID close-path=$CLOSE"
deadline=$((SECONDS+240)); st=timeout
while [ $SECONDS -lt $deadline ]; do
  kill -0 "$APID" 2>/dev/null || { st=app-exited; break; }
  if [ -s "$R/probe.json" ] && python3 -c "import json,sys; sys.exit(0 if 'result' in json.load(open(sys.argv[1])) else 1)" "$R/probe.json" 2>/dev/null; then st=done; break; fi
  sleep 1
done
log "probe-status=$st"
RUN=$(python3 -c "import json,sys;d=json.load(open(sys.argv[1]));print(d.get('runId','') if d.get('result')=='ok' else '')" "$R/probe.json" 2>/dev/null)
SPID=$(server_pid "$DATA"); SCMD=$(ps -o command= -p "${SPID:-0}" 2>/dev/null)
log "run-id=${RUN:-none} server-pid=${SPID:-none}"
case "$SCMD" in "$SRV_EXE serve --data-dir $DATA"*) log "server-verified=yes" ;; *) log "server-verified=no"; kill_exact "$R/kills.txt" "$APID"; exit 3 ;; esac
[ -n "$RUN" ] || { log "abort: no run"; kill_exact "$R/kills.txt" "$APID"; exit 3; }
log "before: $(python3 "$SMOKE/bench-check.py" "$DATA" "$RUN")"
if [ "$SCEN" = close-token ]; then
  log "secret-mode=$(python3 -c "import os,sys;print(oct(os.stat(sys.argv[1]).st_mode & 0o777))" "$R/secret.json") probe-tokenBeforeClose=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['steps'].get('tokenBeforeClose'))" "$R/probe.json")"
  log "token-before-close=$(python3 "$SMOKE/token-check.py" "$R/secret.json")"
fi
osascript -e "tell application id \"$BID\" to activate" >/dev/null 2>&1; sleep 1
MAIN=$(sx 'get name of front window')
log "main-window=$MAIN"
if [ "$CLOSE" != f ]; then
  osascript -e 'tell application "System Events" to keystroke "," using command down' >/dev/null 2>&1; sleep 3
fi
log "windows-before-close=$(sx 'get name of every window')"
case "$CLOSE" in
  a|f) log "action: $(sx "click (first button of window \"$MAIN\" whose subrole is \"AXCloseButton\")")" ;;
  b1) sx "perform action \"AXRaise\" of window \"$MAIN\"" >/dev/null; sleep 1
      log "front=$(sx 'get name of front window')"
      log "action: $(sx 'click menu item "Close Window" of menu "Window" of menu bar 1')" ;;
  b2) log "front=$(sx 'get name of front window')"
      osascript -e 'tell application "System Events" to keystroke "w" using command down' >/dev/null 2>&1 ;;
esac
sleep 3
ALIVE=$(kill -0 "$APID" 2>/dev/null && echo yes || echo no)
log "app-alive-after-close=$ALIVE"
[ "$ALIVE" = yes ] && log "windows-after-close=$(sx 'get name of every window')"
ok=no
for i in $(seq 1 40); do
  out=$(python3 "$SMOKE/bench-check.py" "$DATA" "$RUN"); case "$out" in *'"runListed": false'*) ok=yes; break ;; esac; sleep 0.5
done
log "after: $out"
log "run-removed=$ok"
[ "$SCEN" = close-token ] && log "token-after-close=$(python3 "$SMOKE/token-check.py" "$R/secret.json")"
grep -o 'operation="desktop.retireWindow"[^\n]*' "$DATA/workbench/server/server.log" > "$R/retire.log"; log "retireWindow-calls=$(wc -l < "$R/retire.log" | tr -d ' ')"
grep -n "retire" "$R/app.log" | head -5 >> "$R/meta.txt"
# 정리: 앱이 살아 있으면 AppleScript quit, 그래도 남으면 정확한 PID. 서버는 확인된 PID.
if kill -0 "$APID" 2>/dev/null; then osascript -e "tell application id \"$BID\" to quit" >/dev/null 2>&1; for i in $(seq 1 20); do kill -0 "$APID" 2>/dev/null || break; sleep 0.5; done; fi
kill -0 "$APID" 2>/dev/null && kill_exact "$R/kills.txt" "$APID"
kill_exact "$R/kills.txt" "$SPID"; for i in $(seq 1 60); do kill -0 "$SPID" 2>/dev/null || break; sleep 0.5; done
log "cleanup: app=$(kill -0 "$APID" 2>/dev/null && echo alive || echo gone) server=$(kill -0 "$SPID" 2>/dev/null && echo alive || echo gone)"
