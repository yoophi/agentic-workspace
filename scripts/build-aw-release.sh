#!/usr/bin/env bash

set -euo pipefail

if [[ "${1:-}" == "--" ]]; then
  shift
fi

if [[ $# -ne 1 ]]; then
  echo "usage: pnpm release:build:workbench -- <YYYY.M.PATCH[-rc.N]>" >&2
  exit 2
fi

release_version="$1"
if [[ ! "$release_version" =~ ^[0-9]{4}\.(1[0-2]|[1-9])\.[1-9][0-9]*(-rc\.[1-9][0-9]*)?$ ]]; then
  echo "invalid CALVER release version: $release_version" >&2
  exit 2
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Agentic Workbench release artifacts must be built on macOS." >&2
  exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "$script_dir/.." && pwd)"
cd "$repository_root"

release_tag="v$release_version"
head_commit="$(git rev-parse HEAD)"
if ! git rev-parse -q --verify "refs/tags/$release_tag^{tag}" >/dev/null; then
  echo "$release_tag must exist as an annotated tag before a release build." >&2
  exit 1
fi
tag_commit="$(git rev-list -n 1 "$release_tag" 2>/dev/null || true)"
if [[ -z "$tag_commit" || "$tag_commit" != "$head_commit" ]]; then
  echo "$release_tag must be an annotated tag pointing at HEAD before a release build." >&2
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "tracked files must be clean before a release build." >&2
  exit 1
fi

apple_environment_variables=(
  APPLE_CERTIFICATE
  APPLE_CERTIFICATE_PASSWORD
  APPLE_ID
  APPLE_PASSWORD
  APPLE_TEAM_ID
  APPLE_API_KEY
  APPLE_API_ISSUER
  APPLE_API_KEY_PATH
  APPLE_SIGNING_IDENTITY
)
for apple_environment_variable in "${apple_environment_variables[@]}"; do
  apple_environment_value="$(printenv "$apple_environment_variable" 2>/dev/null || true)"
  if [[ -z "$apple_environment_value" ]]; then
    unset "$apple_environment_variable"
  fi
done

export AGENTIC_WORKBENCH_RELEASE_VERSION="$release_version"
export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
export COMMIT_SHA="$head_commit"
export GITHUB_REF_TYPE="tag"
export GITHUB_REF_NAME="$release_tag"

# 외부 서버 모드가 사용하는 서버도 같은 소스·CALVER로 빌드해 앱 안에 포함한다.
cargo build --locked --release --target aarch64-apple-darwin -p agentic-workbench-server
sidecar_directory="$repository_root/target/release-sidecars"
mkdir -p "$sidecar_directory"
cp "$repository_root/target/aarch64-apple-darwin/release/agentic-workbench-server" \
  "$sidecar_directory/agentic-workbench-server-aarch64-apple-darwin"
# JSON escaping은 경로의 공백/따옴표까지 처리한다.
tauri_override="$(python3 - "$release_version" "$sidecar_directory/agentic-workbench-server" <<'PYCONFIG'
import json, sys
print(json.dumps({"version": sys.argv[1], "bundle": {"externalBin": [sys.argv[2]]}}))
PYCONFIG
)"
pnpm --filter @yoophi/agentic-workbench exec tauri build \
  --target aarch64-apple-darwin \
  --bundles app,dmg \
  --config "$tauri_override"

bundle_root="$repository_root/target/aarch64-apple-darwin/release/bundle"
if [[ ! -d "$bundle_root" ]]; then
  bundle_root="$repository_root/target/release/bundle"
fi

app_path="$bundle_root/macos/Agentic Workbench.app"
dmg_directory="$bundle_root/dmg"
if [[ ! -d "$app_path" || ! -d "$dmg_directory" ]]; then
  echo "Tauri did not produce the expected app and DMG bundle directories." >&2
  exit 1
fi

dmg_count="$(find "$dmg_directory" -maxdepth 1 -type f -name "*_${release_version}_aarch64.dmg" | wc -l | tr -d ' ')"
if [[ "$dmg_count" != "1" ]]; then
  echo "expected one $release_version aarch64 DMG, found $dmg_count" >&2
  exit 1
fi
dmg_path="$(find "$dmg_directory" -maxdepth 1 -type f -name "*_${release_version}_aarch64.dmg" -print)"

app_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app_path/Contents/Info.plist")"
if [[ "$app_version" != "$release_version" ]]; then
  echo "app bundle version mismatch: expected $release_version, got $app_version" >&2
  exit 1
fi

test -x "$app_path/Contents/MacOS/agentic-workbench-server"
file "$app_path/Contents/MacOS/agentic-workbench-server" | grep -q 'arm64'
codesign --verify --strict --verbose=2 "$app_path/Contents/MacOS/agentic-workbench-server"
codesign --verify --deep --strict --verbose=2 "$app_path"
# 데스크톱 기동만으로는 살아 있는 옛 공유 서버 연결을 잡아낼 수 없다.
# 격리된 데이터 경로에서 번들 서버의 실제 광고 버전을 확인한다.
python3 - "$app_path/Contents/MacOS/agentic-workbench-server" "$release_version" <<'PYSERVER'
import json, subprocess, sys, tempfile
with tempfile.TemporaryDirectory(prefix="aw-release-server-") as data_dir:
    server, version = sys.argv[1:]
    try:
        result = subprocess.run([server, "ensure", "--data-dir", data_dir],
                                check=True, capture_output=True, text=True, timeout=30)
        descriptor = json.loads(result.stdout)
        if descriptor["serverVersion"] != version:
            raise RuntimeError("bundled server CALVER does not match the app")
        print("bundled server version:", descriptor["serverVersion"])
    finally:
        subprocess.run([server, "stop", "--data-dir", data_dir],
                       check=True, capture_output=True, timeout=30)
PYSERVER

file "$app_path/Contents/MacOS/agentic-workbench" | grep -q 'arm64'
hdiutil verify "$dmg_path"

mount_directory="$(mktemp -d "${TMPDIR:-/tmp}/aw-release.XXXXXX")"
mounted=0
cleanup() {
  if [[ "$mounted" == "1" ]]; then
    hdiutil detach "$mount_directory" >/dev/null 2>&1 || true
  fi
  rmdir "$mount_directory" >/dev/null 2>&1 || true
}
trap cleanup EXIT

hdiutil attach "$dmg_path" -readonly -nobrowse -mountpoint "$mount_directory" >/dev/null
mounted=1
packaged_app="$mount_directory/Agentic Workbench.app"
test -x "$packaged_app/Contents/MacOS/agentic-workbench-server"
file "$packaged_app/Contents/MacOS/agentic-workbench-server" | grep -q 'arm64'
codesign --verify --strict --verbose=2 "$packaged_app/Contents/MacOS/agentic-workbench-server"
codesign --verify --deep --strict --verbose=2 "$packaged_app"
file "$packaged_app/Contents/MacOS/agentic-workbench" | grep -q 'arm64'
packaged_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$packaged_app/Contents/Info.plist")"
if [[ "$packaged_version" != "$release_version" ]]; then
  echo "DMG app version mismatch: expected $release_version, got $packaged_version" >&2
  exit 1
fi

if [[ "$release_version" != *-* ]]; then
  xcrun stapler validate "$packaged_app"
  spctl -a -vv "$packaged_app"
fi

hdiutil detach "$mount_directory" >/dev/null
mounted=0
rmdir "$mount_directory"
trap - EXIT

artifact_directory="$repository_root/release-artifacts/$release_version"
mkdir -p "$artifact_directory"
artifact_name="Agentic.Workbench_${release_version}_aarch64.dmg"
artifact_path="$artifact_directory/$artifact_name"
cp -p "$dmg_path" "$artifact_path"
checksum="$(shasum -a 256 "$artifact_path" | awk '{print $1}')"
printf '%s  %s\n' "$checksum" "$artifact_name" > "$artifact_path.sha256"

echo "release artifact: $artifact_path"
echo "sha256: $checksum"
