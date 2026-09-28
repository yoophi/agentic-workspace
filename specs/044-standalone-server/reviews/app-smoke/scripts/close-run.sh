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
need "$APID" app-pid; remember "$R" "$APID"; log "app-pid=$APID close-path=$CLOSE"
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
case "$SCMD" in "$SRV_EXE serve --data-dir $DATA"*) log "server-verified=yes"; remember "$R" "$SPID" ;; *) log "server-verified=no"; kill_exact "$R/kills.txt" "$APID"; exit 3 ;; esac
[ -n "$RUN" ] || { log "abort: no run"; kill_exact "$R/kills.txt" "$APID"; exit 3; }
log "before: $(python3 "$SMOKE/bench-check.py" "$DATA" "$RUN")"
if [ "$SCEN" = close-token ]; then
  log "secret-mode=$(python3 -c "import os,sys;print(oct(os.stat(sys.argv[1]).st_mode & 0o777))" "$R/secret.json") probe-tokenBeforeClose=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['steps'].get('tokenBeforeClose'))" "$R/probe.json")"
  log "token-before-close=$(python3 "$SMOKE/token-check.py" "$R/secret.json")"
fi
osascript -e "tell application id \"$BID\" to activate" >/dev/null 2>&1; sleep 1
MAIN=$(sx 'get name of front window')
log "main-window=$MAIN"
invalid_stop() {
  # 전역 키를 보내지 않고 이 시도를 무효로 끝낸다. 정리는 기록한 신원(pid·시작 시각·명령줄)이 같은 이 실행의 pid만 —
  # 번들 id로 quit하지 않는다(`open -n`으로 같은 번들의 다른 인스턴스가 떠 있을 수 있다).
  log "key-not-sent: $1 (attempt invalid)"
  kill_exact "$R/kills.txt" "$APID"
  kill_exact "$R/kills.txt" "$SPID"
  exit 6
}
if [ "$CLOSE" != f ]; then
  send_key_to "$APID" "$BID" "," || invalid_stop "Cmd+, : the target app is not frontmost"
  sleep 3
fi
log "windows-before-close=$(sx 'get name of every window')"
# 닫기 동작의 종료 코드를 보존한다(Codex r10 docs): 실패하면 이 경로를 실행하지 못한 무효 시도다.
ACTRC=0
case "$CLOSE" in
  a|f) ACT=$(sx "click (first button of window \"$MAIN\" whose subrole is \"AXCloseButton\")"); ACTRC=$?; log "action: $ACT (exit $ACTRC)" ;;
  b1) sx "perform action \"AXRaise\" of window \"$MAIN\"" >/dev/null; sleep 1
      log "front=$(sx 'get name of front window')"
      ACT=$(sx 'click menu item "Close Window" of menu "Window" of menu bar 1'); ACTRC=$?; log "action: $ACT (exit $ACTRC)" ;;
  b2) log "front=$(sx 'get name of front window')"
      send_key_to "$APID" "$BID" "w" || invalid_stop "Cmd+W : the target app is not frontmost" ;;
esac
sleep 3
ALIVE=$(kill -0 "$APID" 2>/dev/null && echo yes || echo no)
log "app-alive-after-close=$ALIVE"
WINDOWS=""
[ "$ALIVE" = yes ] && { WINDOWS=$(sx 'get name of every window'); log "windows-after-close=$WINDOWS"; }
# 창 상태 판정: (a)(b1)(b2)는 대상 창만 닫혀 앱과 Settings가 남아야 하고, (f)는 마지막 창이라 앱이 끝나야 한다.
window_verdict "$CLOSE" "$ACTRC" "$ALIVE" "$WINDOWS" | tee -a "$R/meta.txt"; WINRC=${PIPESTATUS[0]}
ok=no
for i in $(seq 1 40); do
  out=$(python3 "$SMOKE/bench-check.py" "$DATA" "$RUN"); case "$out" in *'"runListed": false'*) ok=yes; break ;; esac; sleep 0.5
done
log "after: $out"
log "run-removed=$ok"
# 판정 결과는 정리 전에 보존하고 정리 뒤 종료 코드로 돌려준다(Codex r9 docs): 창 닫기는 그 창 토큰을 폐기해야 한다(401).
# 경로별 기대(Codex r11 docs): 메인 창을 닫는 (a)(b1)(f)는 run 제거·메인 창 토큰 401, Settings를 닫는 (b2)는 run 유지·토큰 200.
case "$CLOSE" in b2) WANT_REMOVED=no; WANT_TOKEN=200 ;; *) WANT_REMOVED=yes; WANT_TOKEN=401 ;; esac
log "expect: run-removed=$WANT_REMOVED token=$WANT_TOKEN"
TOKRC=skip
if [ "$SCEN" = close-token ]; then
  TOK=$(python3 "$SMOKE/token-check.py" "$R/secret.json"); log "token-after-close=$TOK"
  token_verdict "$WANT_TOKEN" "$TOK" | tee -a "$R/meta.txt"; TOKRC=${PIPESTATUS[0]}
fi
grep -o 'operation="desktop.retireWindow"[^\n]*' "$DATA/workbench/server/server.log" > "$R/retire.log"; log "retireWindow-calls=$(wc -l < "$R/retire.log" | tr -d ' ')"
grep -n "retire" "$R/app.log" | head -5 >> "$R/meta.txt"
# 정리: 앱·서버 모두 기록한 신원(pid·시작 시각·명령줄)이 같은 이 실행의 pid만 끝낸다(번들 id quit 없음).
kill -0 "$APID" 2>/dev/null && kill_exact "$R/kills.txt" "$APID"
kill_exact "$R/kills.txt" "$SPID"; for i in $(seq 1 60); do kill -0 "$SPID" 2>/dev/null || break; sleep 0.5; done
log "cleanup: app=$(kill -0 "$APID" 2>/dev/null && echo alive || echo gone) server=$(kill -0 "$SPID" 2>/dev/null && echo alive || echo gone)"
# 최종 결과: run이 남으면 7, 토큰 미폐기·검사 오류면 8(정리를 모두 마친 뒤에도 비정상 종료).
close_final "$ok" "$TOKRC" "$WINRC" "$WANT_REMOVED" | tee -a "$R/meta.txt"; FINAL=${PIPESTATUS[0]}
exit "$FINAL"
