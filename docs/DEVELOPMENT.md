# Dunkerfy

A native macOS notification-sound utility written entirely in **Rust**. AppKit supplies the windows, controls, file pickers, menu bar item, and audio playback. There is no webview, Electron, Swift, or background service to install.

## Use the app

1. Build with `mise run build`, then open `dist/Dunkerfy.app`. For everyday use, move it to `/Applications` **before granting permissions**.
2. In Dunkerfy’s **Settings**, open **Accessibility Settings** and enable Dunkerfy. If it is not listed, use the system panel’s **+** button and choose the app. macOS requires you to grant this permission yourself.
3. In macOS **Notifications** settings, leave banners or alerts enabled for your source apps, but disable their original notification sound. Some apps also have their own sound setting. Dunkerfy adds replacement audio; it cannot intercept or cancel an app’s original audio.
4. Create a **Sound rule**. Choose an application or enter the exact source name shown on its notification banner. Optionally match text such as a person, channel, or keyword. Pick a sound, adjust its volume and cooldown, and save.
5. Send a real notification from that app. **Activity** shows whether the event matched, was paused, hit a cooldown, or failed playback.

**Test rule** is explicitly a simulated event: it tests the current editor’s matching and audio without needing Accessibility permission. It does not prove that live system notification capture works. Preview and test actions intentionally work while listening is paused.

### Features

- Per-app rules, optional case-insensitive substring matching, and enabled/disabled rules.
- Ordered priority: the first enabled matching rule wins. A rule in cooldown does not fall through to another rule.
- Per-rule and master volume; per-rule cooldown from 0–3,600 seconds.
- macOS system sounds and imported AIFF, WAV, MP3, M4A, and CAF files (up to 20 MB).
- Imported audio is copied into the app’s library, so moving the original does not break a rule.
- Sound preview, rule simulation, reordering, deletion, and protection against deleting sounds used by rules.
- Atomic settings writes; invalid/corrupt settings are preserved and reported instead of silently replaced.
- Unsaved-edit confirmation when navigating away from the editor.
- Menu bar pause/resume and quit; closing the window leaves monitoring active.
- Optional launch at login via macOS `SMAppService`, controlled by the user.
- A bounded, in-memory activity log. Notification bodies are hidden by default and never persisted.
- Duplicate bundled launches activate the running instance instead of starting another sound monitor.

## Build and develop — mise only

Requirements: macOS 13+, Apple Command Line Tools or Xcode, and [mise](https://mise.jdx.dev/). No Homebrew is used.

```sh
mise trust
mise install
mise run test
mise run check
mise run build
open dist/Dunkerfy.app
```

`mise.toml` pins Rust 1.99.0 and installs rustfmt and clippy through mise. The Rust backend uses rustup internally; its toolchain and Cargo state are confined to the ignored project-local `.mise/` directory. There is no standalone `~/.cargo` or `~/.rustup` installation required. Rust library dependencies are declared in `Cargo.toml` and locked in `Cargo.lock`; Cargo runs through mise.

Useful checks:

```sh
mise exec -- cargo fmt --check
mise exec -- cargo test
mise exec -- cargo clippy --all-targets -- -D warnings
mise exec -- cargo run -- --smoke-test
```

The native smoke test constructs all four pages and the rule editor without writing settings or playing audio. It requires a logged-in macOS GUI session. The unit tests cover matching, priority, cooldown, pause, disabled rules, notification parsing, validation, atomic persistence, corruption preservation, and audio copying.

The bundle script targets the current machine’s architecture and ad-hoc signs the app for local use. To distribute outside your machine, use your own Developer ID via `DUNKERFY_SIGNING_IDENTITY`, then notarize and staple with your Apple developer credentials. An ad-hoc signature is not a notarized public release. Rebuilding or relocating an ad-hoc signed app may require removing and re-adding its Accessibility permission.

## How notification capture works

The worker uses `AXIsProcessTrusted`, `AXUIElementCreateApplication`, and `AXUIElementCopyAttributeValue` from Apple’s Accessibility APIs to inspect the Notification Center process. It checks for visible `AXNotificationCenterBanner` / `AXNotificationCenterAlert` subtrees every 400 ms. Traversal is bounded and runs off the AppKit thread. It neither queries the notification database nor opens Notification Center history.

Existing banners establish a silent baseline on launch/reconnection. Banner accessibility identities are retained while visible, preventing the same persistent alert from replaying every scan. New banners get a short settling period before parsing. The worker reconnects if Notification Center restarts and keeps checking for permission changes.

macOS does **not** expose a supported public subscription API for notifications from all other apps. Accessibility is a practical integration, but Notification Center’s private UI structure and descriptions can change between macOS versions. The parser fails closed on an unknown description instead of guessing an app from message text. This needs live testing on each macOS release you intend to support.

Limitations:

- Notifications suppressed by Focus, notifications without banners, or banners too brief to observe may not trigger.
- Hidden previews limit keyword matching; only accessible text can be matched.
- The app name is matched exactly (ignoring case), not by a substring. Localized app names or helper-app names may need to be entered manually using the observed name in Activity.
- Notification Center does not reliably expose source bundle identifiers. The selected bundle ID is retained, but matching falls back to the banner’s app name when the event has no authoritative ID.
- Changes to the contents of an already-visible accessibility element do not replay that same element. If macOS reuses an element for a distinct notification without removing it, that event can be missed.
- One sound plays at a time; a new matching event replaces current playback.
- Launch at login requires a proper app bundle and can require approval in macOS Login Items settings.

## Data and privacy

Settings: `~/Library/Application Support/Dunkerfy/settings.json`

Imported sounds: `~/Library/Application Support/Dunkerfy/Sounds/`

Notification content is processed locally for matching. Activity retains at most 60 entries in memory, never on disk. If you opt into showing notification text, turning the option off clears the text already retained. No application code sends network requests or analytics.

## Source map

- `src/model.rs`: serializable settings, validation, matching, and cooldown engine.
- `src/storage.rs`: persistence and sound library operations.
- `src/monitor.rs`: thread-confined Core Foundation/Accessibility bindings and banner monitoring.
- `src/mac.rs`: AppKit UI, audio, login items, and event handling.
- `tests/logic.rs`: deterministic logic and filesystem tests.
- `Resources/Info.plist`, `scripts/build-app.sh`: native bundle and signing.

Technical references: [Apple Accessibility API](https://developer.apple.com/documentation/applicationservices/axuielement_h), [objc2](https://docs.rs/objc2/), [mise Rust backend](https://mise.jdx.dev/lang/rust.html). Dunkerfy is an independent implementation, not affiliated with Chirpy. No Chirpy code or assets are used.
