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
