use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use ecs::Resource;
use serde::{Deserialize, Serialize};
use terminal::Color;
use widgets2::{PromptColors, ScrollbarColors, list2::ListColors};

use crate::database::TrackRating;

// TODO: Add show_scrollbars to config.

const VERSION: u8 = 0;

#[derive(Debug, Default, Resource)]
pub struct Settings {
    config: Config,
    path: Option<PathBuf>,
}

impl Settings {
    pub fn new(path: PathBuf) -> Self {
        Self {
            config: Config::default(),
            path: Some(path),
        }
    }

    pub fn read(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        Ok(Self {
            config: Config::read(path)?,
            path: Some(path.to_path_buf()),
        })
    }

    pub const fn config(&self) -> &Config {
        &self.config
    }

    pub const fn set_config(&mut self, config: &Config) {
        self.config.general.skip_rating = config.general.skip_rating;
        self.config.general.keep_selected = config.general.keep_selected;
        self.config.general.search_by_path = config.general.search_by_path;
        self.config.colors.primary = config.colors.primary;
        self.config.colors.secondary = config.colors.secondary;
        self.config.colors.neutral = config.colors.neutral;
    }

    pub const fn general(&self) -> &General {
        &self.config.general
    }

    pub const fn general_mut(&mut self) -> &mut General {
        &mut self.config.general
    }

    pub const fn skip_rating(&self) -> TrackRating {
        self.config.general.skip_rating
    }

    pub const fn keep_selected(&self) -> bool {
        self.config.general.keep_selected
    }

    pub const fn search_by_path(&self) -> bool {
        self.config.general.search_by_path
    }

    pub const fn colors(&self) -> &Colors {
        &self.config.colors
    }

    pub const fn colors_mut(&mut self) -> &mut Colors {
        &mut self.config.colors
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };

        self.config.save(path)
    }

    pub fn hash(&self) -> u64 {
        self.config.hash()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u8,
    pub general: General,
    pub colors: Colors,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: VERSION,
            general: Default::default(),
            colors: Default::default(),
        }
    }
}

impl Config {
    fn read(path: &Path) -> Result<Self, ConfigError> {
        let bytes = std::fs::read(path).map_err(|err| ConfigError::from_read(path, err))?;

        #[derive(Deserialize)]
        struct V {
            version: u8,
        }

        let V { version } =
            toml::from_slice(&bytes).map_err(|err| ConfigError::from_de(path, err))?;

        let config: Self = match version {
            VERSION => toml::from_slice(&bytes).map_err(|err| ConfigError::from_de(path, err))?,
            _ => Err(ConfigError::from_unknown_version(path, version))?,
        };

        Ok(config)
    }

    fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let toml = toml::to_string(self).map_err(|err| ConfigError::from_ser(path, err))?;
        std::fs::write(path, &toml).map_err(|err| ConfigError::from_write(path, err))?;

        Ok(())
    }

    pub fn hash(&self) -> u64 {
        utils::hasher_fast(|hasher| {
            hasher.write_u8(self.general.skip_rating as u8);
            hasher.write_u8(self.general.keep_selected as u8);
            hasher.write_u8(self.general.search_by_path as u8);
            self.colors.primary.hash(hasher);
            self.colors.secondary.hash(hasher);
            self.colors.neutral.hash(hasher);
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct General {
    #[serde(rename = "skip_tracks_with_rating")]
    pub skip_rating: TrackRating,
    #[serde(rename = "keep_selected_track_on_sort")]
    pub keep_selected: bool,
    #[serde(rename = "search_by_path")]
    pub search_by_path: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            skip_rating: TrackRating::None,
            keep_selected: false,
            search_by_path: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Colors {
    pub primary: Color,
    pub secondary: Color,
    pub neutral: Color,

    #[serde(skip)]
    pub normal: Color,
    #[serde(skip)]
    pub background: Color,
    #[serde(skip)]
    pub red: Color,
}

impl Colors {
    pub const fn all(color: Color) -> Self {
        Self {
            primary: color,
            secondary: color,
            neutral: color,
            normal: color,
            background: color,
            red: color,
        }
    }

    pub const fn list(&self) -> ListColors {
        ListColors {
            scrollbar: self.scrollbar(),
        }
    }

    pub const fn prompt(&self) -> PromptColors {
        PromptColors {
            normal: self.normal,
            placeholder: self.neutral,
            disabled: self.neutral,
        }
    }

    pub const fn scrollbar(&self) -> ScrollbarColors {
        ScrollbarColors {
            thumb: self.neutral,
            track: None,
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            primary: Color::YellowBright,
            secondary: Color::Yellow,
            neutral: Color::Indexed(240),
            normal: Color::Default,
            background: Color::Default,
            red: Color::Red,
        }
    }
}

#[derive(Debug)]
pub struct ConfigError {
    message: String,
    source: Option<ConfigErrorSource>,
}

impl ConfigError {
    pub const fn new(message: String, source: Option<ConfigErrorSource>) -> Self {
        Self { message, source }
    }

    fn from_read(path: &Path, err: std::io::Error) -> Self {
        Self {
            message: format!("Failed to read settings from '{}'.", path.display()),
            source: Some(ConfigErrorSource::Io(err)),
        }
    }

    fn from_write(path: &Path, err: std::io::Error) -> Self {
        Self {
            message: format!("Failed to write settings to '{}'.", path.display()),
            source: Some(ConfigErrorSource::Io(err)),
        }
    }

    fn from_de(path: &Path, err: toml::de::Error) -> Self {
        Self {
            message: format!("Failed to deserialize settings from '{}'.", path.display()),
            source: Some(ConfigErrorSource::De(err)),
        }
    }

    fn from_ser(path: &Path, err: toml::ser::Error) -> Self {
        Self {
            message: format!("Failed to write settings to '{}'.", path.display()),
            source: Some(ConfigErrorSource::Ser(err)),
        }
    }

    fn from_unknown_version(path: &Path, version: u8) -> Self {
        Self {
            message: format!(
                "Failed to deserialize settings from '{}' due to unknown version: {version}.",
                path.display()
            ),
            source: None,
        }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|source| match source {
            ConfigErrorSource::Io(err) => err as _,
            ConfigErrorSource::De(err) => err as _,
            ConfigErrorSource::Ser(err) => err as _,
        })
    }
}

#[derive(Debug)]
pub enum ConfigErrorSource {
    Io(std::io::Error),
    De(toml::de::Error),
    Ser(toml::ser::Error),
}

impl From<String> for ConfigError {
    fn from(message: String) -> Self {
        Self {
            message,
            source: None,
        }
    }
}

impl From<&str> for ConfigError {
    fn from(message: &str) -> Self {
        Self {
            message: String::from(message),
            source: None,
        }
    }
}
