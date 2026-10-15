# In-app updates

Dunkerfy 0.2.0 embeds Sparkle 2.10.0. Rust loads the framework from the app's own `Contents/Frameworks` directory and calls its Objective-C API; no Swift or Objective-C application source is required. `mise install` installs the pinned Sparkle distribution and signing utilities. Direct `cargo run` builds can still run the application, but updates require the packaged `.app`.

The Updates page provides manual checks, automatic checks, and automatic download/installation. Both automatic settings default off. Sparkle owns their persistence in the app's user defaults. Disabling automatic checks also disables automatic downloads. Notification matching works offline. Update requests go to Cloudflare and GitHub and never contain notification text, rules, or sounds; system-profile reporting is disabled.

`Resources/Info.plist` pins the public Ed25519 key and the HTTPS feed URL. Both feed signatures and archive signatures are required, with verification before extraction. The private key lives in the login Keychain under Sparkle account `app.dunkerfy.mac`; never commit or upload an export. Arrange a secure backup using Sparkle's documented key-export process before relying on this Mac as the only release machine. Losing the key can prevent updates.

The first updater-enabled release must be installed manually by existing 0.1.0 users. Keep the bundle ID `app.dunkerfy.mac` and app name `Dunkerfy.app` stable. Increment both Cargo/package marketing version and `CFBundleVersion` for every release. Rules and copied sounds live outside the app bundle and survive replacement.

## Apple distribution setup

1. In Xcode → Settings → Accounts, sign in with the paid Apple Developer account. Select the correct team and choose Manage Certificates → + → **Developer ID Application**. Apple requires the Account Holder for creating this certificate. An Apple Development or Apple Distribution certificate is not a substitute. Alternatively create the certificate in the [Developer portal](https://developer.apple.com/help/account/certificates/create-developer-id-certificates/) using a CSR generated on this Mac.
2. Confirm the identity is installed with its private key using `security find-identity -v -p codesigning`.
3. Create an app-specific password in the Apple Account website, then run `xcrun notarytool store-credentials Dunkerfy --apple-id YOUR_APPLE_ID --team-id YOUR_TEAM_ID` in your own terminal. Enter the app-specific password at its hidden prompt, not in chat, command arguments, source files, or shell history. The tool validates and stores credentials in Keychain. On first use, macOS may also ask you to allow Sparkle’s `generate_appcast` and `sign_update` tools to access their update-signing key; approve the specific tools on the Mac. An App Store Connect API key is an alternative.
4. Put only the non-secret certificate name and profile name in ignored `mise.local.toml`:

```toml
[env]
DUNKERFY_SIGNING_IDENTITY = "Developer ID Application: Your Name (TEAMID)"
DUNKERFY_NOTARY_PROFILE = "Dunkerfy"
```

`mise run release-prepare` requires these settings, signs the framework and its helpers inside-out with hardened runtime and timestamps, submits to Apple, requires Accepted status, staples and validates the ticket, and assesses the app with Gatekeeper. Only then does it create the final ZIP and signed appcast. A local unnotarized test package can be prepared explicitly with `mise run release-prepare -- --preview`; that flag never means a notarized build.

## Release procedure

1. Bump versions, write `docs/releases/vVERSION.md`, and update signing/capture caveats to match what was actually verified. Run `mise run release-prepare`. It runs tests, lint, packaged UI construction and updater startup checks. It refuses to overwrite an existing version's release directory.
2. Review `dist/releases/VERSION/`. Run an actual update from a previous installed build on a test Mac, including cancel, offline/failure behavior, unsaved edits, install/relaunch and persistence of rules/sounds. Construction and signature tests alone do not prove replacement/relaunch works.
3. Commit and push the source. Publish `Dunkerfy-VERSION-macOS-arm64.zip` and `SHA256SUMS` in GitHub release `vVERSION`. The appcast points to this exact immutable release asset. Keep previews marked as prereleases until live notification testing is complete.
4. Download the public ZIP and verify its checksum and signature. Copy the already-signed `appcast.xml` into `website/appcast.xml` and update the website download links. Deploy via `oleary-ops` as documented in DEPLOYMENT.md. **Archive first, feed last.** Do not hand-edit signed XML or release notes; regenerate/re-sign them.
5. Fetch the live feed and verify it with `sign_update --account app.dunkerfy.mac --verify downloaded-appcast.xml`. Run `node scripts/check-update.mjs downloaded-appcast.xml downloaded.zip` against the pinned app public key. Confirm a clean installed copy finds the expected version.

The initial feed intentionally contains one current full archive and no deltas. GitHub retains older releases. A bad update should be withdrawn from the feed and followed by a fixed build with a higher bundle version; don't overwrite an already-published version or assume automatic downgrades are safe.

Sources: [Sparkle integration](https://sparkle-project.org/documentation/), [programmatic API](https://sparkle-project.org/documentation/programmatic-setup/), [Apple notarization](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow).
