use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use terminal::Color;
use widgets2::{ListColors, PromptColors, ScrollbarColors};

use crate::AppDirRef;

// TODO: Add show_scrollbars to config.

pub type SettingsResult<T> = Result<T, ConfigError>;

const VERSION: u8 = 0;

#[derive(Debug, Default)]
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
                Err(err) => return Err(*err)?,
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
        self.config.general.desired_retention = config.general.desired_retention;
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

    pub const fn desired_retention_as_fraction(&self) -> f32 {
        self.config.general.desired_retention as f32 / 100.0
    }

    pub const fn colors(&self) -> &Colors {
        &self.config.colors
    }

    pub const fn colors_mut(&mut self) -> &mut Colors {
        &mut self.config.colors
    }

    pub fn save(&self) -> SettingsResult<()> {
        let Some(path) = self.path.as_ref() else {
            return Err("No path to save settings to.")?;
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
            _ => Err("Unknown version.")?,
        };

        Ok(config)
    }

    fn save(&self, path: &Path) -> SettingsResult<()> {
        let toml = toml::to_string(self)?;
        std::fs::write(path, &toml)?;

        Ok(())
    }

    pub fn hash(&self) -> u64 {
        utils::hasher_fast(|hasher| {
            hasher.write_u8(self.general.desired_retention);
            self.colors.primary.hash(hasher);
            self.colors.secondary.hash(hasher);
            self.colors.neutral.hash(hasher);
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct General {
    #[serde(rename = "desired_retention")]
    pub desired_retention: u8,
}

impl Default for General {
    fn default() -> Self {
        Self {
            desired_retention: 80,
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
    Other(&'static str),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => err.fmt(f),
            Self::De(err) => err.fmt(f),
            Self::Ser(err) => err.fmt(f),
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for ConfigError {}

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

impl From<&'static str> for ConfigError {
    fn from(err: &'static str) -> Self {
        Self::Other(err)
    }
}
