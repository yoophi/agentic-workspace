#!/usr/bin/env bash
# Build the actual merged044 server from an immutable archive, never the current tree.
set -euo pipefail
case "${1:-}" in
  ""|--build-only) ;;
  *) echo "usage: $0 [--build-only]" >&2; exit 2 ;;
esac
if [[ "$(uname -s)" != Darwin ]]; then
  echo "047 actual wire validation is scoped to macOS" >&2
  exit 2
fi
AW047_REPO_ROOT="$(git rev-parse --show-toplevel)"
AW047_SERVER_COMMIT=20fcd5fdcf633ae06792d51a9b963e3857909440
git -C "$AW047_REPO_ROOT" cat-file -e "$AW047_SERVER_COMMIT^{commit}"
AW047_BUILD_ROOT="$(mktemp -d /private/tmp/aw-047-wire.XXXXXX)"
chmod 700 "$AW047_BUILD_ROOT"
mkdir "$AW047_BUILD_ROOT/source"
git -C "$AW047_REPO_ROOT" archive --format=tar --output="$AW047_BUILD_ROOT/source.tar" "$AW047_SERVER_COMMIT"
tar -xf "$AW047_BUILD_ROOT/source.tar" -C "$AW047_BUILD_ROOT/source"
(
  cd "$AW047_BUILD_ROOT/source"
  CARGO_TARGET_DIR="$AW047_BUILD_ROOT/target" cargo build --locked -p agentic-workbench-server
) >"$AW047_BUILD_ROOT/server-build.log" 2>&1 || {
  echo "exact merged044 server build failed; log: $AW047_BUILD_ROOT/server-build.log" >&2
  exit 1
}
AW047_SERVER_BINARY="$AW047_BUILD_ROOT/target/debug/agentic-workbench-server"
python3 - "$AW047_SERVER_COMMIT" "$AW047_BUILD_ROOT" "$AW047_SERVER_BINARY" <<'PY'
import hashlib, json, pathlib, platform, subprocess, sys
commit, root, binary = sys.argv[1:]
root = pathlib.Path(root)
def digest(path):
    with open(path, 'rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()
manifest = dict(serverCommit=commit, binary=binary, binarySha256=digest(binary),
                archiveSha256=digest(root/'source.tar'), lockSha256=digest(root/'source/Cargo.lock'),
                buildCommand='cargo build --locked -p agentic-workbench-server', buildExit=0,
                os=platform.platform(), macOS=subprocess.check_output(['sw_vers','-productVersion'],text=True).strip())
(root/'provenance.json').write_text(json.dumps(manifest, indent=2)+'\n')
print(json.dumps(manifest))
PY
if [[ "${1:-}" != --build-only ]]; then
  cd "$AW047_REPO_ROOT"
  AW_047_SERVER_BINARY="$AW047_SERVER_BINARY" AW_047_SERVER_PROVENANCE="$AW047_BUILD_ROOT/provenance.json" AW_047_WIRE_EVIDENCE="$AW047_BUILD_ROOT/wire-evidence.json" \
    cargo test -p aw-cli --test actual_server -- --ignored --nocapture
fi
