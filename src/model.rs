use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Sound {
    Bundled(String),
    System(String),
    Imported(String),
}

impl Default for Sound {
    fn default() -> Self {
        Self::Bundled("bubble".into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: Uuid,
    pub name: String,
    pub app_name: String,
    pub bundle_id: Option<String>,
    pub contains: String,
    pub sound: Sound,
    pub volume: f32,
    pub cooldown_secs: u32,
    pub enabled: bool,
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New sound rule".into(),
            app_name: String::new(),
            bundle_id: None,
            contains: String::new(),
            sound: Sound::default(),
            volume: 0.7,
            cooldown_secs: 1,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub app_name: String,
    pub bundle_id: Option<String>,
    pub text: String,
}

impl Rule {
    pub fn matches(&self, notification: &Notification) -> bool {
        if !self.enabled || self.app_name.trim().is_empty() {
            return false;
        }
        let app_matches = match (&self.bundle_id, &notification.bundle_id) {
            (Some(expected), Some(actual)) => expected == actual,
            _ => self.app_name.trim().to_lowercase() == notification.app_name.trim().to_lowercase(),
        };
        app_matches
            && notification
                .text
                .to_lowercase()
                .contains(&self.contains.trim().to_lowercase())
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Give the rule a name.".into());
        }
        if self.app_name.trim().is_empty() {
            return Err("Choose an app or enter its exact notification display name.".into());
        }
        if !(0.0..=1.0).contains(&self.volume) {
            return Err("Volume must be between 0 and 100%.".into());
        }
        if self.cooldown_secs > 3600 {
            return Err("Cooldown must be at most 3,600 seconds.".into());
        }
        validate_sound(&self.sound)
    }
}

pub fn validate_sound(sound: &Sound) -> Result<(), String> {
    let name = match sound {
        Sound::Bundled(s) => {
            return crate::catalog::preset(s)
                .map(|_| ())
                .ok_or_else(|| "Unknown bundled sound.".into());
        }
        Sound::System(s) | Sound::Imported(s) => s,
    };
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err("Invalid sound filename.".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub rules: Vec<Rule>,
    pub paused: bool,
    pub master_volume: f32,
    pub show_notification_text: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            rules: Vec::new(),
            paused: false,
            master_volume: 1.0,
            show_notification_text: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported settings version. Keep this file and use a compatible Dunkerfy version.".into());
        }
        if !(0.0..=1.0).contains(&self.master_volume) {
            return Err("Invalid master volume.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for rule in &self.rules {
            rule.validate()?;
            if !ids.insert(rule.id) {
                return Err("Duplicate rule IDs in settings.".into());
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Engine {
    last_played: HashMap<Uuid, Instant>,
}
#[derive(Debug, PartialEq)]
pub enum Decision {
    Paused,
    NoMatch,
    Cooldown(Uuid),
    Play(Uuid),
}
impl Engine {
    pub fn evaluate(
        &mut self,
        notification: &Notification,
        settings: &Settings,
        now: Instant,
    ) -> Decision {
        if settings.paused {
            return Decision::Paused;
        }
        let Some(rule) = settings
            .rules
            .iter()
            .find(|rule| rule.matches(notification))
        else {
            return Decision::NoMatch;
        };
        if self.last_played.get(&rule.id).is_some_and(|last| {
            now.saturating_duration_since(*last) < Duration::from_secs(rule.cooldown_secs.into())
        }) {
            return Decision::Cooldown(rule.id);
        }
        self.last_played.insert(rule.id, now);
        Decision::Play(rule.id)
    }
    pub fn reset(&mut self) {
        self.last_played.clear();
    }
}

/// Accessibility exposes a comma-separated description beginning with the source app.
/// Unknown layouts fail closed instead of guessing an app from message content.
pub fn parse_banner(description: &str, texts: &[String]) -> Option<Notification> {
    let (app, rest) = description.split_once(", ")?;
    if app.trim().is_empty() {
        return None;
    }
    let text = texts
        .iter()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let text = if text.is_empty() {
        rest.to_owned()
    } else {
        text
    };
    if text.trim().is_empty() {
        return None;
    }
    Some(Notification {
        app_name: app.trim().to_owned(),
        bundle_id: None,
        text,
    })
}
