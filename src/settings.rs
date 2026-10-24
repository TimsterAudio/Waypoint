//! Waypoint's own settings (not the trip's): the look, recent trips, and
//! whether to reach the internet for exchange rates.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 3] = [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];

    pub fn label(self) -> &'static str {
        match self {
            ThemeChoice::System => "Match system",
            ThemeChoice::Light => "Light",
            ThemeChoice::Dark => "Dark",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: ThemeChoice,
    /// Trips opened lately, newest first.
    pub recent: Vec<PathBuf>,
    /// Write the phone itinerary beside the trip each time it's saved.
    pub phone_page: bool,
    /// Fetch exchange rates when a trip opens. Off until switched on: no
    /// network unless asked.
    pub rates_on_open: bool,
    /// Suggest places from OpenStreetMap while typing a name. Off until
    /// switched on: no network unless asked.
    pub places_online: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: ThemeChoice::System,
            recent: Vec::new(),
            phone_page: true,
            rates_on_open: false,
            places_online: false,
        }
    }
}

const MAX_RECENT: usize = 8;

impl Settings {
    /// Where the settings live: `~/.config/waypoint/settings.json` on Linux,
    /// `%APPDATA%\Waypoint\settings.json` on Windows.
    pub fn path() -> Option<PathBuf> {
        let name = if cfg!(windows) {
            "Waypoint"
        } else {
            "waypoint"
        };
        dirs::config_dir().map(|d| d.join(name).join("settings.json"))
    }

    pub fn load(path: Option<&Path>) -> Settings {
        path.and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: Option<&Path>) -> Result<(), String> {
        let Some(path) = path else { return Ok(()) };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        waypoint_core::file::write_atomic(path, &bytes).map_err(|e| e.to_string())
    }

    pub fn remember(&mut self, trip: &Path) {
        self.recent.retain(|p| p != trip);
        self.recent.insert(0, trip.to_path_buf());
        self.recent.truncate(MAX_RECENT);
    }

    pub fn forget(&mut self, trip: &Path) {
        self.recent.retain(|p| p != trip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_keeps_newest_first_without_repeats() {
        let mut s = Settings::default();
        for n in 0..10 {
            s.remember(Path::new(&format!("/t/{n}.trip.json")));
        }
        s.remember(Path::new("/t/5.trip.json"));
        assert_eq!(s.recent.len(), MAX_RECENT);
        assert_eq!(s.recent[0], Path::new("/t/5.trip.json"));
        assert_eq!(
            s.recent
                .iter()
                .filter(|p| p.ends_with("5.trip.json"))
                .count(),
            1
        );
    }

    #[test]
    fn old_settings_files_load() {
        let s: Settings = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(s.theme, ThemeChoice::Dark);
        assert!(s.phone_page);
    }
}
