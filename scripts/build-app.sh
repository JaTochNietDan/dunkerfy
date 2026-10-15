#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
export MACOSX_DEPLOYMENT_TARGET=13.0
cargo build --release --locked
app="dist/Dunkerfy.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/dunkerfy "$app/Contents/MacOS/dunkerfy"
cp Resources/Info.plist "$app/Contents/Info.plist"
iconset="dist/AppIcon.iconset"
mkdir -p "$iconset"
target/release/dunkerfy --write-icon dist/AppIcon.png
for size in 16 32 128 256 512; do
    /usr/bin/sips -z "$size" "$size" dist/AppIcon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    retina=$((size * 2))
    /usr/bin/sips -z "$retina" "$retina" dist/AppIcon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
/usr/bin/iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"
/usr/bin/codesign --force --sign "${DUNKERFY_SIGNING_IDENTITY:--}" "$app"
/usr/bin/codesign --verify --deep --strict "$app"
echo "Built $PWD/$app"
