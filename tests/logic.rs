use dunkerfy::{model::*, storage::Storage};
use std::{
    fs,
    time::{Duration, Instant},
};
use uuid::Uuid;
fn rule(app: &str, keyword: &str) -> Rule {
    Rule {
        name: "Work".into(),
        app_name: app.into(),
        contains: keyword.into(),
        ..Rule::default()
    }
}
fn event(app: &str, text: &str) -> Notification {
    Notification {
        app_name: app.into(),
        text: text.into(),
        bundle_id: None,
    }
}
#[test]
fn exact_app_and_keyword_matching() {
    let r = rule("Slack", "URGENT");
    assert!(r.matches(&event("slack", "An urgent message")));
    assert!(!r.matches(&event("Slack Helper", "urgent")));
    assert!(!r.matches(&event("Slack", "ordinary")));
    assert!(!rule("", "").matches(&event("", "test")));
}
#[test]
fn bundle_identity_wins() {
    let mut r = rule("Mail", "");
    r.bundle_id = Some("com.apple.mail".into());
    let mut e = event("Mail", "message");
    e.bundle_id = Some("other.mail".into());
    assert!(!r.matches(&e));
    e.bundle_id = Some("com.apple.mail".into());
    e.app_name = "Localized Mail".into();
    assert!(r.matches(&e));
}
#[test]
fn priority_cooldown_and_pause() {
    let first = rule("Slack", "urgent");
    let second = rule("Slack", "");
    let mut s = Settings {
        rules: vec![first.clone(), second],
        ..Settings::default()
    };
    let mut engine = Engine::default();
    let now = Instant::now();
    let e = event("Slack", "urgent");
    assert_eq!(engine.evaluate(&e, &s, now), Decision::Play(first.id));
    assert_eq!(
        engine.evaluate(&e, &s, now + Duration::from_millis(500)),
        Decision::Cooldown(first.id)
    );
    assert_eq!(
        engine.evaluate(&e, &s, now + Duration::from_secs(1)),
        Decision::Play(first.id)
    );
    s.paused = true;
    assert_eq!(
        engine.evaluate(&e, &s, now + Duration::from_secs(3)),
        Decision::Paused
    );
    s.paused = false;
    assert_eq!(
        engine.evaluate(&e, &s, now + Duration::from_secs(3)),
        Decision::Play(first.id)
    );
}
#[test]
fn disabled_and_unmatched_rules() {
    let mut r = rule("Slack", "");
    r.enabled = false;
    let s = Settings {
        rules: vec![r],
        ..Settings::default()
    };
    assert_eq!(
        Engine::default().evaluate(&event("Slack", "hello"), &s, Instant::now()),
        Decision::NoMatch
    );
    assert_eq!(
        Engine::default().evaluate(&event("Mail", "hello"), &s, Instant::now()),
        Decision::NoMatch
    );
}
#[test]
fn parser_handles_unknown_layouts_and_commas() {
    let n = parse_banner("Slack, Jane, hello, world", &[]).unwrap();
    assert_eq!(n.app_name, "Slack");
    assert_eq!(n.text, "Jane, hello, world");
    assert_eq!(
        parse_banner("Mail, sender", &["Subject".into(), "Body".into()])
            .unwrap()
            .text,
        "Subject\nBody"
    );
    assert!(parse_banner("", &[]).is_none());
    assert!(parse_banner("Unknown layout", &["Some text".into()]).is_none());
    assert!(parse_banner(", message", &[]).is_none());
}
#[test]
fn rejects_traversal_duplicate_ids_and_bad_values() {
    assert!(validate_sound(&Sound::Imported("../../etc/passwd".into())).is_err());
    assert!(validate_sound(&Sound::System("/Glass".into())).is_err());
    let mut r = rule("Mail", "");
    r.volume = f32::NAN;
    assert!(r.validate().is_err());
    r.volume = 0.5;
    r.cooldown_secs = 3601;
    assert!(r.validate().is_err());
    r.cooldown_secs = 1;
    assert!(Settings {
        rules: vec![r.clone(), r],
        ..Settings::default()
    }
    .validate()
    .is_err());
    assert!(Settings {
        version: 99,
        ..Settings::default()
    }
    .validate()
    .is_err());
}
struct Fixture(Storage);
impl Fixture {
    fn new() -> Self {
        Self(Storage {
            root: std::env::temp_dir().join(format!("dunkerfy-test-{}", Uuid::new_v4())),
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0.root);
    }
}
#[test]
fn roundtrip_and_corrupt_settings_preserved() {
    let f = Fixture::new();
    assert!(f.0.load().unwrap().rules.is_empty());
    let s = Settings {
        rules: vec![rule("Mail", "")],
        ..Settings::default()
    };
    f.0.save(&s).unwrap();
    assert_eq!(f.0.load().unwrap().rules[0].id, s.rules[0].id);
    assert_eq!(fs::read_dir(&f.0.root).unwrap().count(), 1);
    fs::write(f.0.root.join("settings.json"), "broken").unwrap();
    assert!(f.0.load().is_err());
    assert_eq!(
        fs::read_to_string(f.0.root.join("settings.json")).unwrap(),
        "broken"
    );
}
#[test]
fn audio_import_copies_file() {
    let f = Fixture::new();
    fs::create_dir_all(&f.0.root).unwrap();
    let source = f.0.root.join("my sound.wav");
    fs::write(&source, b"test audio bytes").unwrap();
    let sound = f.0.import_sound(&source).unwrap();
    fs::remove_file(source).unwrap();
    assert_eq!(
        fs::read(f.0.sound_path(&sound).unwrap()).unwrap(),
        b"test audio bytes"
    );
    assert!(f.0.sounds().iter().any(|(_, s)| s == &sound));
    let invalid = f.0.root.join("not-audio.txt");
    fs::write(&invalid, "text").unwrap();
    assert!(f.0.import_sound(&invalid).is_err());
}
