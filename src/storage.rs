use crate::model::{validate_sound, Settings, Sound};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct Storage {
    pub root: PathBuf,
}
impl Storage {
    pub fn standard() -> Result<Self, String> {
        let home = std::env::var_os("HOME").ok_or("HOME is unavailable")?;
        Ok(Self {
            root: PathBuf::from(home).join("Library/Application Support/Dunkerfy"),
        })
    }
    pub fn load(&self) -> Result<Settings, String> {
        let path = self.root.join("settings.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(e) => return Err(format!("Cannot read {}: {e}", path.display())),
        };
        let settings: Settings = serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "Settings could not be read; your file has been preserved at {}. {e}",
                path.display()
            )
        })?;
        settings.validate()?;
        Ok(settings)
    }
    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
        let temp = self.root.join(format!("settings-{}.tmp", Uuid::new_v4()));
        let result = (|| -> std::io::Result<()> {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, self.root.join("settings.json"))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result.map_err(|e| format!("Could not save settings: {e}"))
    }
    pub fn import_sound(&self, source: &Path) -> Result<Sound, String> {
        let metadata = fs::metadata(source).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > 20 * 1024 * 1024 {
            return Err("Choose an audio file smaller than 20 MB.".into());
        }
        let ext = source
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["aiff", "aif", "wav", "mp3", "m4a", "caf"].contains(&ext.as_str()) {
            return Err("Choose AIFF, WAV, MP3, M4A, or CAF audio.".into());
        }
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Sound");
        let clean: String = stem
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
            .take(60)
            .collect();
        let filename = format!("{clean}--{}.{}", Uuid::new_v4(), ext);
        let dir = self.root.join("Sounds");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        fs::copy(source, dir.join(&filename)).map_err(|e| e.to_string())?;
        Ok(Sound::Imported(filename))
    }
    pub fn sound_path(&self, sound: &Sound) -> Result<PathBuf, String> {
        validate_sound(sound)?;
        Ok(match sound {
            Sound::Bundled(id) => {
                let executable = std::env::current_exe().map_err(|e| e.to_string())?;
                let parent = executable.parent().ok_or("Missing executable directory")?;
                let resources = if parent.file_name().is_some_and(|n| n == "MacOS") {
                    parent
                        .parent()
                        .ok_or("Missing bundle directory")?
                        .join("Resources/Sounds")
                } else {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Resources/Sounds")
                };
                resources.join(format!("{id}.wav"))
            }
            Sound::System(name) => {
                PathBuf::from("/System/Library/Sounds").join(format!("{name}.aiff"))
            }
            Sound::Imported(name) => self.root.join("Sounds").join(name),
        })
    }
    pub fn sounds(&self) -> Vec<(String, Sound)> {
        let mut result: Vec<_> = crate::catalog::PRESETS
            .iter()
            .map(|p| (p.name.clone(), Sound::Bundled(p.id.clone())))
            .collect();
        let mut system = Vec::new();
        if let Ok(entries) = fs::read_dir("/System/Library/Sounds") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "aiff") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        system.push((name.to_string(), Sound::System(name.to_string())));
                    }
                }
            }
        }
        system.sort_by(|a, b| a.0.cmp(&b.0));
        result.extend(system);
        if let Ok(entries) = fs::read_dir(self.root.join("Sounds")) {
            let mut imported = Vec::new();
            for entry in entries.flatten() {
                if entry.path().is_file() {
                    let filename = entry.file_name().to_string_lossy().into_owned();
                    let label = filename.split("--").next().unwrap_or(&filename).to_owned();
                    imported.push((format!("{label} · imported"), Sound::Imported(filename)));
                }
            }
            imported.sort_by(|a, b| a.0.cmp(&b.0));
            result.extend(imported);
        }
        result
    }
}
