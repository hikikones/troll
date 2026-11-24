use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use ecs::Resource;
use serde::{Deserialize, Serialize};
use shared::env::AppDirRef;
use terminal::Color;
use widgets::{ListColors, PromptColors, ScrollbarColors};

use crate::database::TrackRating;

// TODO: Add show_scrollbars to config.

pub type SettingsResult<T = ()> = Result<T, ConfigError>;

const VERSION: u8 = 0;

#[derive(Debug, Default, Resource)]
pub struct Settings {
    config: Config,
    path: Option<PathBuf>,
}

impl Settings {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            config: Config::default(),
            path,
        }
    }

    pub fn read_or_create(path: Option<&Path>, app_dir: AppDirRef) -> SettingsResult<Self> {
        #[cfg(debug_assertions)]
        if path.is_none() {
            return Ok(Settings::default());
        }

        let file = match path {
            Some(path) => path.to_path_buf(),
            None => match app_dir {
                Ok(dir) => dir.join("settings.toml"),
                Err(err) => return Err(ConfigError::NoAppPath(err)),
            },
        };

        if file.exists() {
            return Ok(Self {
                config: Config::read(&file)?,
                path: Some(file),
            });
        }

        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let config = Config::default();
        config.save(&file)?;

        Ok(Self {
            config,
            path: Some(file),
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

    pub fn save(&self) -> SettingsResult {
        let Some(path) = self.path.as_ref() else {
            return Err(ConfigError::NoSavePath);
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
    fn read(path: &Path) -> SettingsResult<Self> {
        let bytes = std::fs::read(path)?;

        #[derive(Deserialize)]
        struct V {
            version: u8,
        }

        let V { version } = toml::from_slice(&bytes)?;

        let config: Self = match version {
            VERSION => toml::from_slice(&bytes)?,
            _ => return Err(ConfigError::UnknownVersion),
        };

        Ok(config)
    }

    fn save(&self, path: &Path) -> SettingsResult {
        let toml = toml::to_string(self)?;
        std::fs::write(path, &toml)?;

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
pub enum ConfigError {
    Io(std::io::Error),
    De(toml::de::Error),
    Ser(toml::ser::Error),
    NoAppPath(&'static str),
    NoSavePath,
    UnknownVersion,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => err.fmt(f),
            Self::De(err) => err.fmt(f),
            Self::Ser(err) => err.fmt(f),
            Self::NoAppPath(err) => f.write_str(err),
            Self::NoSavePath => f.write_str("No path to save settings to."),
            Self::UnknownVersion => f.write_str("Unknown version."),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => err.source(),
            Self::De(err) => err.source(),
            Self::Ser(err) => err.source(),
            Self::NoAppPath(_) => None,
            Self::NoSavePath => None,
            Self::UnknownVersion => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        Self::De(err)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(err: toml::ser::Error) -> Self {
        Self::Ser(err)
    }
}
