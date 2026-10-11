//! Portable, versionable user preferences.
//! Filesystem location and persistence are delegated to the desktop adapter.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EditorSettings {
    pub theme: Theme,
    pub font_size: u16,
    pub show_sidebar: bool,
    pub show_status_bar: bool,
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
        }
    }
}

impl EditorSettings {
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
    fn invalid_font_size_is_rejected() {
        let settings = EditorSettings::parse("font_size = 2").unwrap();
        assert!(settings.validate().is_err());
    }
}
