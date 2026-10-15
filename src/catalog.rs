use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Debug, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub duration: f64,
}

pub static PRESETS: LazyLock<Vec<Preset>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../Resources/Sounds/catalog.json"))
        .expect("The bundled sound catalog must be valid")
});

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}
