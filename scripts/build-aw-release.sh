#!/usr/bin/env bash

set -euo pipefail

if [[ "${1:-}" == "--" ]]; then
  shift
fi

if [[ $# -ne 1 ]]; then
  echo "usage: pnpm release:build:workbench -- <YYYY.M.D[-rc.N]>" >&2
  exit 2
fi

release_version="$1"
if [[ ! "$release_version" =~ ^[0-9]{4}\.(1[0-2]|[1-9])\.([1-9]|[12][0-9]|3[01])(-rc\.[1-9][0-9]*)?$ ]]; then
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

export AGENTIC_WORKBENCH_RELEASE_VERSION="$release_version"
export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
export COMMIT_SHA="$head_commit"
export GITHUB_REF_TYPE="tag"
export GITHUB_REF_NAME="$release_tag"

tauri_override="{\"version\":\"$release_version\"}"
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

codesign --verify --deep --strict --verbose=2 "$app_path"
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
