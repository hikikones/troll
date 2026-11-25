use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use ecs::Resource;
use serde::{Deserialize, Serialize};
use shared::env::NoAppDir;
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

    pub fn read_or_create(path: Option<&Path>) -> SettingsResult<Self> {
        #[cfg(debug_assertions)]
        if path.is_none() {
            return Ok(Settings::default());
        }

        let Some(path) = path else {
            return Self::from_app_dir();
        };

        let config = Config::read(path)?;

        Ok(Self {
            config,
            path: Some(path.to_path_buf()),
        })
    }

    fn from_app_dir() -> SettingsResult<Self> {
        let mut path = shared::env::get_app_dir(
            crate::APP_QUALIFIER,
            crate::APP_ORGANIZATION,
            crate::APP_NAME,
        )
        .map_err(ConfigError::NoAppDir)?;

        path.push("settings.toml");

        if path.exists() {
            return Ok(Self {
                config: Config::read(&path)?,
                path: Some(path),
            });
        }

        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            return Err(ConfigError::CreateAppDirs { path, error });
        }

        let config = Config::default();
        config.save(&path)?;

        Ok(Self {
            config,
            path: Some(path),
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
        let bytes = std::fs::read(path).map_err(|error| ConfigError::Read {
            path: path.to_path_buf(),
            error,
        })?;

        #[derive(Deserialize)]
        struct V {
            version: u8,
        }

        let V { version } = toml::from_slice(&bytes).map_err(|error| ConfigError::Deserialize {
            path: path.to_path_buf(),
            error,
        })?;

        let config: Self = match version {
            VERSION => toml::from_slice(&bytes).map_err(|error| ConfigError::Deserialize {
                path: path.to_path_buf(),
                error,
            })?,
            _ => return Err(ConfigError::UnknownVersion),
        };

        Ok(config)
    }

    fn save(&self, path: &Path) -> SettingsResult {
        let toml = toml::to_string(self).map_err(ConfigError::Serialize)?;
        std::fs::write(path, &toml).map_err(|error| ConfigError::Save {
            path: path.to_path_buf(),
            error,
        })?;

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
    Read {
        path: PathBuf,
        error: std::io::Error,
    },
    Save {
        path: PathBuf,
        error: std::io::Error,
    },
    Deserialize {
        path: PathBuf,
        error: toml::de::Error,
    },
    Serialize(toml::ser::Error),
    CreateAppDirs {
        path: PathBuf,
        error: std::io::Error,
    },
    NoAppDir(NoAppDir),
    NoSavePath,
    UnknownVersion,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, .. } => f.write_fmt(format_args!(
                "Failed to read settings from '{}'.",
                path.display()
            )),
            Self::Save { path, .. } => f.write_fmt(format_args!(
                "Failed to save settings to '{}'.",
                path.display()
            )),
            Self::Deserialize { path, .. } => f.write_fmt(format_args!(
                "Failed to deserialize settings from '{}'.",
                path.display()
            )),
            Self::Serialize(_) => f.write_str("Failed to serialize settings."),
            Self::CreateAppDirs { path, .. } => f.write_fmt(format_args!(
                "Failed to create app directory: '{}'.",
                path.display()
            )),
            Self::NoAppDir(err) => err.fmt(f),
            Self::NoSavePath => f.write_str("No path to save settings to."),
            Self::UnknownVersion => f.write_str("Unknown version."),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { error, .. } => Some(error),
            Self::Save { error, .. } => Some(error),
            Self::Deserialize { error, .. } => Some(error),
            Self::Serialize(error) => Some(error),
            Self::CreateAppDirs { error, .. } => Some(error),
            Self::NoAppDir(err) => err.source(),
            Self::NoSavePath => None,
            Self::UnknownVersion => None,
        }
    }
}
