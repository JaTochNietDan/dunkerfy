#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mode="${1:---notarized}"
if [[ "$mode" != "--preview" && "$mode" != "--notarized" ]]; then
    echo "Usage: mise run release-prepare [--notarized|--preview]" >&2; exit 1
fi
if [[ "$mode" == "--notarized" ]]; then
    : "${DUNKERFY_SIGNING_IDENTITY:?Set your Developer ID Application identity}"
    : "${DUNKERFY_NOTARY_PROFILE:?Set your notarytool Keychain profile name}"
    if [[ "$DUNKERFY_SIGNING_IDENTITY" != "Developer ID Application:"* ]]; then
        echo "A Developer ID Application certificate is required." >&2; exit 1
    fi
fi
version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' Resources/Info.plist)
release="dist/releases/$version"
if [[ -e "$release" ]]; then
    echo "$release already exists. Preserve published archives; use a new version or move an unpublished build aside." >&2; exit 1
fi
account="app.dunkerfy.mac"
public_key=$(generate_keys --account "$account" -p)
expected_key=$(/usr/libexec/PlistBuddy -c 'Print :SUPublicEDKey' Resources/Info.plist)
if [[ "$public_key" != "$expected_key" ]]; then
    echo "Keychain signing key does not match the app's pinned public key." >&2; exit 1
fi
test -f "docs/releases/v$version.md"
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
bash scripts/build-app.sh
node scripts/test-updates.mjs
dist/Dunkerfy.app/Contents/MacOS/dunkerfy --smoke-test
dist/Dunkerfy.app/Contents/MacOS/dunkerfy --updater-smoke-test
mkdir -p dist/releases
stage=$(mktemp -d "dist/.release-$version.XXXXXX")
trap 'echo "Release incomplete; diagnostic files retained at $stage" >&2' EXIT
archive="$stage/Dunkerfy-$version-macOS-arm64.zip"
ditto -c -k --sequesterRsrc --keepParent dist/Dunkerfy.app "$archive"
if [[ "$mode" == "--notarized" ]]; then
    xcrun notarytool submit "$archive" --keychain-profile "$DUNKERFY_NOTARY_PROFILE" \
        --wait --output-format json > "$stage/notarization.json"
    node -e 'const fs=require("fs"); if(JSON.parse(fs.readFileSync(process.argv[1])).status!=="Accepted") process.exit(1)' "$stage/notarization.json"
    xcrun stapler staple dist/Dunkerfy.app
    xcrun stapler validate dist/Dunkerfy.app
    spctl --assess --type execute --verbose dist/Dunkerfy.app
    rm "$archive"
    ditto -c -k --sequesterRsrc --keepParent dist/Dunkerfy.app "$archive"
fi
cp "docs/releases/v$version.md" "$stage/Dunkerfy-$version-macOS-arm64.md"
generate_appcast --account "$account" --maximum-deltas 0 --embed-release-notes \
    --download-url-prefix "https://github.com/JaTochNietDan/dunkerfy/releases/download/v$version/" \
    --link https://dunkerfy.oleary.club "$stage"
sign_update --account "$account" --verify "$stage/appcast.xml"
node scripts/check-update.mjs "$stage/appcast.xml" "$archive"
(cd "$stage" && shasum -a 256 "Dunkerfy-$version-macOS-arm64.zip" > SHA256SUMS)
mv "$stage" "$release"
trap - EXIT
echo "Prepared $mode release in $release. Publish the archive before deploying appcast.xml."
