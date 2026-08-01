#!/bin/sh
set -eu
case "${1:-}" in ????.*.*|????.*.*-rc.*) ;; *) echo "usage: $0 YYYY.M.D[-rc.N]" >&2; exit 2;; esac
MA_CALVER="$1" MA_GIT_COMMIT="$(git rev-parse --short=12 HEAD)" MA_GIT_TAG="$(git describe --tags --exact-match 2>/dev/null || true)" pnpm tauri build --config src-tauri/tauri.release.conf.json --config "{\"version\":\"$1\"}"
