#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS is required" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_file="$repo_root/crates/process-supervisor/tests/fixtures/macos_endpoint_security_probe.c"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/aw-045-es.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

probe="$work_dir/aw-045-endpoint-security-probe"
xcrun --sdk macosx clang \
  -Wall \
  -Wextra \
  -Werror \
  -fblocks \
  "$source_file" \
  -lEndpointSecurity \
  -o "$probe"

if ! otool -L "$probe" | grep -q 'libEndpointSecurity'; then
  echo "compiled probe does not link libEndpointSecurity" >&2
  exit 1
fi

runtime_result="$($probe)"
printf 'ordinary_%s\n' "$runtime_result"
if sudo -n true 2>/dev/null; then
  elevated_result="$(sudo -n "$probe")"
  printf 'elevated_%s\n' "$elevated_result"
else
  printf 'elevated_probe=unavailable\n'
fi

bundle="$work_dir/AWProcessObserver.systemextension"
mkdir -p "$bundle/Contents/MacOS"
cp "$probe" "$bundle/Contents/MacOS/AWProcessObserver"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>AWProcessObserver</string>
  <key>CFBundleIdentifier</key>
  <string>io.agenticworkspace.process-observer.feasibility</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>AW Process Observer Feasibility</string>
  <key>CFBundlePackageType</key>
  <string>SYSX</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>NSExtension</key>
  <dict>
    <key>NSExtensionPointIdentifier</key>
    <string>com.apple.system-extension.endpoint-security</string>
  </dict>
</dict>
</plist>
PLIST

plutil -lint "$bundle/Contents/Info.plist" >/dev/null
codesign --force --sign - "$bundle"
codesign --verify --strict "$bundle"

identity_count="$({ security find-identity -v -p codesigning 2>/dev/null || true; } | grep -Ec '^[[:space:]]+[0-9]+\)' || true)"
profile_count=0
for profile_root in \
  "$HOME/Library/MobileDevice/Provisioning Profiles" \
  "$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"; do
  [[ -d "$profile_root" ]] || continue
  while IFS= read -r -d '' profile; do
    profile_plist="$work_dir/profile.plist"
    if security cms -D -i "$profile" > "$profile_plist" 2>/dev/null && \
      /usr/libexec/PlistBuddy -c 'Print :Entitlements:com.apple.developer.endpoint-security.client' "$profile_plist" 2>/dev/null | grep -qx 'true'; then
      profile_count=$((profile_count + 1))
    fi
  done < <(find "$profile_root" -type f \( -name '*.mobileprovision' -o -name '*.provisionprofile' \) -print0)
done

printf 'sdk_link=true bundle_shape=true adhoc_signature=true codesigning_identities=%s endpoint_security_profiles=%s\n' \
  "$identity_count" "$profile_count"
printf 'production_activation_attempted=false reason=requires_matching_apple_identity_profile_application_install_user_approval_and_tcc\n'
