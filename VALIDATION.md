# Validation — 2026-10-15

Environment: Apple Silicon macOS, Rust 1.99.0 installed and invoked through mise.

Passed:

- `cargo test`: 8 integration tests covering matching, priority, cooldown, pause, disabled rules, parser fallback, path validation, settings persistence/corruption handling, and audio copying.
- `cargo clippy --all-targets -- -D warnings`.
- `cargo fmt` and successful compilation after final changes.
- Native AppKit smoke test: all four pages, rule editor, status menu, and login-service query constructed successfully.
- UI inspection of the release app and rule editor through macOS Accessibility and screenshots.
- UI test with an unsaved Mail/urgent rule: simulated event matched and the activity log reported “Simulated test · sound played.”
- Final release bundle built; Info.plist lint and strict code-signature verification passed.
- Apple Silicon ZIP created at `dist/Dunkerfy-macOS-arm64.zip`.
- Verified that no standalone `~/.cargo` or `~/.rustup` installation remains. Toolchain state is under this project’s `.mise/`, managed by mise.

Not yet verified:

- Real third-party notification capture, including Notification Center’s current AX layout, deduplication, and reconnect behavior. Dunkerfy did not have Accessibility permission during UI testing.
- Login-at-login registration and an actual login cycle. The option is off by default; no user login preference was changed during testing.
- Native file-picker import flow end to end. File-copy logic is tested; AppKit decoding/playback was tested with a system sound.
- Intel macOS builds, older macOS versions, or a notarized distributable release.
- UI interaction with the final unsaved-edit confirmation and duplicate-launch behavior. These compile and the updated native construction smoke test passed; the Mac locked before further interaction was possible.

The earlier test instance was stopped after building the final bundle. Unlock the Mac, open the final app, grant Accessibility access using the in-app Settings instructions, and trigger a banner from an app with a configured rule to complete live integration testing.

## Publication verification

Published on 2026-10-15:

- Public repository: https://github.com/JaTochNietDan/dunkerfy. GitHub Actions are disabled.
- Developer preview: https://github.com/JaTochNietDan/dunkerfy/releases/tag/v0.1.0. Downloaded the public ZIP and verified it against the published SHA256SUMS.
- Website: https://dunkerfy.oleary.club, Cloudflare Worker `oleary-dunkerfy`, version `b79fd6e7-eb2d-43d2-a2df-6ecec53f1d21`. All six public assets match local source byte for byte; security headers verified.
- Local browser inspection confirmed the landing page layout and click-to-play sound demo. App screenshots are intentionally omitted at the owner's request while the Mac is locked.
- Reviewed Terraform plan added exactly one custom domain, with no updates or deletions. The full post-apply plan is clean. All 39 pre-existing DNS records were unchanged.
- The oleary.club homepage now includes Dunkerfy in its preview collection. Published index matches the generated output.

Publication does not change the live-capture, signing, or platform limitations listed above.
