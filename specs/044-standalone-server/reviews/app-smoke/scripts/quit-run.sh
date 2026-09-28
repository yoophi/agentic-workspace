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
if [ "${BUSY:-}" = 1 ]; then AGENT="$AGENT --end-turn-gate $R/turn.gate --after-gate-chunk --gate-limit 600"; SCEN=quit-busy; PHASE=ready-to-quit-busy; fi
# TOKEN=1(Codex 코드 리뷰): close-token probe가 이 창 토큰을 비밀 파일(0600)에 넘긴다. 종료 뒤 같은 토큰이 거절되는지 본다.
if [ "${TOKEN:-}" = 1 ]; then
  if [ "${BUSY:-}" = 1 ]; then SCEN=quit-busy-token; PHASE=ready-to-quit-busy-token; else SCEN=close-token; PHASE=ready-to-close; fi
fi
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
# 종료 동작을 실제로 보냈는가(보낸 명령의 종료 코드 0). PID 소멸만으로는 그 경로의 증거가 아니다(Codex r7 docs).
SENT=no
case "$QUIT" in
  c) # 키 입력은 앞 프로세스로 간다 — 이 앱이 실제로 앞에 올 때까지 조건 대기(상한 10초) 뒤 보낸다.
     front=no; for i in $(seq 1 40); do
       osascript -e "tell application id \"$BID\" to activate" >/dev/null 2>&1
       osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $APID) to true" >/dev/null 2>&1
       [ "$(osascript -e 'tell application "System Events" to get unix id of first process whose frontmost is true' 2>/dev/null)" = "$APID" ] && { front=yes; break; }; sleep 0.25
     done; log "frontmost-before-cmd-q=$front"
     if [ "$front" != yes ]; then
       # 키 입력은 앞 프로세스로 가는 전역 입력이다 — 대상 앱이 앞이 아니면 다른 앱을 끌 수 있으므로 보내지 않는다.
       log "cmd-q-not-sent: the target app is not frontmost (attempt invalid)"
     else
       # 보내기 직전 한 번 더 확인한다(그 사이 앞 창이 바뀌면 보내지 않음).
       if [ "$(osascript -e 'tell application "System Events" to get unix id of first process whose frontmost is true' 2>/dev/null)" = "$APID" ]; then
         osascript -e 'tell application "System Events" to keystroke "q" using command down' >> "$R/quit.txt" 2>&1 && SENT=yes
       else
         log "cmd-q-not-sent: frontmost changed just before sending (attempt invalid)"
       fi
     fi ;;
  d) # Dock 타일은 이름으로 고른다 — 그 이름의 앱 프로세스가 정확히 하나이고 그것이 이 실행의 APID일 때만 누른다(다른 앱을 끄지 않게).
     # System Events의 프로세스 이름은 실행 파일 이름이라(설치본 AW와 같음) 쓰지 않는다. 표시 이름으로 pid를 찾는다.
     same=$("$SMOKE/apps-named" "$PRODUCT" 2>/dev/null)
     log "dock-name-pids=${same:-none}"
     if [ "$same" != "$APID" ]; then
       log "dock-quit-not-sent: the Dock name does not map to exactly this app (attempt invalid)"
     else
       osascript -e "tell application \"System Events\" to tell process \"Dock\" to tell UI element \"$PRODUCT\" of list 1 to perform action \"AXShowMenu\"" >> "$R/quit.txt" 2>&1
       # 메뉴가 열릴 때까지 조건 대기(상한 10초). 열리지 않으면 누르지 않고 무효로 기록한다.
       menu=no; for i in $(seq 1 40); do
         [ "$(osascript -e "tell application \"System Events\" to tell process \"Dock\" to exists menu 1 of UI element \"$PRODUCT\" of list 1" 2>/dev/null)" = true ] && { menu=yes; break; }; sleep 0.25
       done; log "dock-menu-open=$menu"
       if [ "$menu" = yes ]; then
         osascript -e "tell application \"System Events\" to tell process \"Dock\" to tell UI element \"$PRODUCT\" of list 1 to click menu item \"Quit\" of menu 1" >> "$R/quit.txt" 2>&1 && SENT=yes
       else
         log "dock-quit-not-sent: the Dock menu did not open (attempt invalid)"
       fi
     fi ;;
  e) # AppleScript quit은 번들 id로 간다 — 그 번들의 실행 중 인스턴스가 정확히 이 APID 하나일 때만 보낸다.
     same=$("$SMOKE/apps-named" --bundle "$BID" 2>/dev/null)
     log "bundle-pids=${same:-none}"
     if [ "$same" = "$APID" ]; then
       osascript -e "tell application id \"$BID\" to quit" >> "$R/quit.txt" 2>&1 && SENT=yes
     else
       log "applescript-quit-not-sent: the bundle id does not map to exactly this app (attempt invalid)"
     fi ;;
  g) # SIGTERM도 기록한 신원(시작 시각·명령줄)이 같을 때만 보낸다. 대상이 사라졌거나 신원이 다르면 보내지 않고 무효.
     if send_sigterm "$R/kills.txt" "$APID"; then SENT=yes; else log "sigterm-not-sent: the target identity is gone or changed (attempt invalid)"; fi ;;
  *) log "unknown path"; exit 4 ;;
esac
gone=no
for i in $(seq 1 60); do kill -0 "$APID" 2>/dev/null || { gone=yes; break; }; sleep 0.5; done
log "app-gone=$gone"
log "quit-action-sent=$SENT"
# 판정: 종료 동작을 보냈고(SENT=yes) 그 뒤 PID가 사라졌을 때만 이 경로의 증거다. 아니면 정리한 뒤에도 무효이고 비정상 종료 코드로 끝난다.
quit_verdict "$SENT" "$gone" "$QUIT" | tee -a "$R/meta.txt"; VERDICT=${PIPESTATUS[0]}
[ "$gone" != yes ] && { log "app still running after quit path; stopping exact pid"; kill_exact "$R/kills.txt" "$APID"; }
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
OC=$?
log "owner-check-exit=$OC"
cat "$R/owner-check.json"
# 서버를 정확한 PID로 정지(확인된 경우만).
case "$SCMD" in "$SRV_EXE serve --data-dir $DATA"*) kill_exact "$R/kills.txt" "$SPID" ;; esac
for i in $(seq 1 60); do kill -0 "$SPID" 2>/dev/null || break; sleep 0.5; done
log "server-stopped=$(kill -0 "$SPID" 2>/dev/null && echo no || echo yes)"
# 최종 결과(Codex r8 docs): 경로 무효는 정리를 모두 마친 뒤에도 5, 유효한 종료 뒤 run 지속 확인(owner-check) 실패는 6, 둘 다
# 통과해야 0.
smoke_final "$VERDICT" "$OC" | tee -a "$R/meta.txt"; FINAL=${PIPESTATUS[0]}
exit "$FINAL"
