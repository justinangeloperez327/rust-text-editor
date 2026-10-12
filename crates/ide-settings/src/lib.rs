//! Portable, versionable user preferences.
//! Filesystem location and persistence are delegated to the desktop adapter.

use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EditorSettings {
    pub theme: Theme,
    pub font_size: u16,
    pub show_sidebar: bool,
    pub show_status_bar: bool,
    pub show_panel: bool,
    pub sidebar_width: u16,
    pub panel_height: u16,
    pub sidebar_view: String,
    pub active_panel_tab: String,
    pub panel_dock: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    Dark,
    Light,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            font_size: 14,
            show_sidebar: true,
            show_status_bar: true,
            show_panel: true,
            sidebar_width: 240,
            panel_height: 180,
            sidebar_view: "explorer".to_owned(),
            active_panel_tab: "terminal".to_owned(),
            panel_dock: "bottom".to_owned(),
        }
    }
}

impl EditorSettings {
    /// Load settings from disk, or return defaults when the file does not exist.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(Box::new(error)),
        };
        let settings = Self::parse(&content)?;
        settings.validate().map_err(io::Error::other)?;
        Ok(settings)
    }

    /// Persist preferences using TOML; creates the configuration directory.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.validate().map_err(io::Error::other)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_toml()?)?;
        Ok(())
    }

    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !(8..=72).contains(&self.font_size) {
            return Err("font_size must be between 8 and 72");
        }
        if !(160..=600).contains(&self.sidebar_width) {
            return Err("sidebar_width must be between 160 and 600");
        }
        if !(100..=480).contains(&self.panel_height) {
            return Err("panel_height must be between 100 and 480");
        }
        if !matches!(
            self.sidebar_view.as_str(),
            "explorer" | "search" | "source-control" | "run" | "extensions"
        ) {
            return Err("sidebar_view is not recognized");
        }
        if !matches!(
            self.active_panel_tab.as_str(),
            "problems" | "output" | "debug-console" | "terminal"
        ) {
            return Err("active_panel_tab is not recognized");
        }
        if !matches!(self.panel_dock.as_str(), "bottom" | "right" | "left") {
            return Err("panel_dock is not recognized");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        EditorSettings::default().validate().unwrap();
    }

    #[test]
    fn round_trip() {
        let settings = EditorSettings::default();
        let encoded = settings.to_toml().unwrap();
        assert_eq!(EditorSettings::parse(&encoded).unwrap(), settings);
    }

    #[test]
    fn partial_settings_apply_defaults() {
        let settings = EditorSettings::parse("font_size = 18").unwrap();
        assert_eq!(settings.font_size, 18);
        assert_eq!(settings.theme, Theme::Dark);
    }

    #[test]
    fn invalid_layout_dimensions_are_rejected() {
        let settings = EditorSettings::parse("sidebar_width = 20").unwrap();
        assert!(settings.validate().is_err());
        let settings = EditorSettings::parse("panel_height = 900").unwrap();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn invalid_font_size_is_rejected() {
        let settings = EditorSettings::parse("font_size = 2").unwrap();
        assert!(settings.validate().is_err());
    }
}
