#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${repo_root}"

app_name="SimpleTerm"
bundle_id="com.simple-term.app"
binary_path="target/release/simple-term"
icon_source="apps/simple-term/assets/SimpleTerm.icns"
dist_dir="dist"
app_root="${dist_dir}/${app_name}.app"
stage_dir="${dist_dir}/dmg-stage"
dmg_name="simple-term-local-preview.dmg"

bundle_version="$(
  awk '
    /^\[workspace\.package\]/ { in_section = 1; next }
    in_section && /^\[/ { in_section = 0 }
    in_section && /^version[[:space:]]*=/ {
      gsub(/"/, "", $3);
      print $3 "-local";
      exit
    }
  ' Cargo.toml
)"

if [[ -z "${bundle_version}" ]]; then
  echo "Could not read [workspace.package].version from Cargo.toml"
  exit 1
fi

if [[ ! -f "${icon_source}" ]]; then
  echo "Missing icon asset: ${icon_source}"
  exit 1
fi

cargo build --locked --release -p simple-term-app

mkdir -p "${dist_dir}"
rm -rf "${app_root}" "${stage_dir}" "${dist_dir}/${dmg_name}"
mkdir -p "${app_root}/Contents/MacOS" "${app_root}/Contents/Resources"

cat > "${app_root}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>CFBundleName</key>
    <string>${app_name}</string>
    <key>CFBundleDisplayName</key>
    <string>${app_name}</string>
    <key>CFBundleIdentifier</key>
    <string>${bundle_id}</string>
    <key>CFBundleVersion</key>
    <string>${bundle_version}</string>
    <key>CFBundleShortVersionString</key>
    <string>${bundle_version}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>simple-term</string>
    <key>CFBundleIconFile</key>
    <string>SimpleTerm.icns</string>
  </dict>
</plist>
PLIST

cp "${binary_path}" "${app_root}/Contents/MacOS/simple-term"
cp "${icon_source}" "${app_root}/Contents/Resources/SimpleTerm.icns"
chmod +x "${app_root}/Contents/MacOS/simple-term"

mkdir -p "${stage_dir}"
cp -R "${app_root}" "${stage_dir}/"
ln -s /Applications "${stage_dir}/Applications"

hdiutil create \
  -volname "simple-term local-preview" \
  -srcfolder "${stage_dir}" \
  -ov \
  -format UDZO \
  "${dist_dir}/${dmg_name}" || {
  echo "Warning: failed to create DMG via hdiutil; app bundle is still available at ${app_root}" >&2
  rm -rf "${stage_dir}"
  echo "Packaged app: ${app_root}"
  exit 0
}

rm -rf "${stage_dir}"

echo "Packaged app: ${app_root}"
echo "Packaged dmg: ${dist_dir}/${dmg_name}"
