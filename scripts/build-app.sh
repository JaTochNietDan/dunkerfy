#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
export MACOSX_DEPLOYMENT_TARGET=13.0
cargo build --release --locked
app="dist/Dunkerfy.app"
sparkle="$(mise where github:sparkle-project/Sparkle)"
test -d "$sparkle/Sparkle.framework"
# Recreate only our generated bundle, so old helpers cannot survive a rebuild.
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$app/Contents/Frameworks"
cp target/release/dunkerfy "$app/Contents/MacOS/dunkerfy"
cp Resources/Info.plist "$app/Contents/Info.plist"
/usr/bin/ditto "$sparkle/Sparkle.framework" "$app/Contents/Frameworks/Sparkle.framework"
cp "$sparkle/LICENSE" "$app/Contents/Resources/Sparkle-LICENSE.txt"
iconset="dist/AppIcon.iconset"
mkdir -p "$iconset"
target/release/dunkerfy --write-icon dist/AppIcon.png
for size in 16 32 128 256 512; do
    /usr/bin/sips -z "$size" "$size" dist/AppIcon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    retina=$((size * 2))
    /usr/bin/sips -z "$retina" "$retina" dist/AppIcon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
/usr/bin/iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"
identity="${DUNKERFY_SIGNING_IDENTITY:--}"
if [[ "$identity" != "-" ]]; then
    # Sign nested code inside-out, preserving Sparkle helper entitlements.
    framework="$app/Contents/Frameworks/Sparkle.framework/Versions/B"
    for component in "$framework/XPCServices/Downloader.xpc" \
        "$framework/XPCServices/Installer.xpc" "$framework/Autoupdate" \
        "$framework/Updater.app" "$app/Contents/Frameworks/Sparkle.framework"; do
        /usr/bin/codesign --force --sign "$identity" --options runtime --timestamp \
            --preserve-metadata=identifier,entitlements "$component"
    done
    /usr/bin/codesign --force --sign "$identity" --options runtime --timestamp "$app"
else
    /usr/bin/codesign --force --sign - "$app"
fi
/usr/bin/codesign --verify --deep --strict "$app"
echo "Built $PWD/$app"
