use std::path::{Path, PathBuf};

use bevystov::{
    app::App,
    database::Database,
    jukebox::{AudioPlayer, Jukebox},
    mpris::MediaControls,
    settings::{ConfigError, ConfigErrorSource, Settings},
};
use terminal::Terminal;

fn main() -> std::process::ExitCode {
    let args: Args = clap::Parser::parse();

    if let Err(err) = run(args) {
        eprintln!("Error: {err}");

        let mut source = err.source();
        while let Some(cause) = source {
            eprintln!("Caused by: {cause}");
            source = cause.source();
        }

        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let terminal = Terminal::new()?;

    let jukebox = Jukebox::new(AudioPlayer::new()?);
    let database = Database::new(args.dir);
    let settings = read_settings(args.settings)?;

    let media_controls = if args.media_controls {
        Some(MediaControls::spawn()?)
    } else {
        None
    };

    let mut app = App::new(database, jukebox, settings, media_controls);

    terminal.enter(|term| app.run(term))
}

#[derive(Debug, clap::Parser)]
#[command(version, about, styles = CLAP_STYLING)]
struct Args {
    /// The directory for your music.
    #[arg(value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    dir: std::path::PathBuf,

    /// Optional path for your settings file. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "FILE.toml", value_hint = clap::ValueHint::FilePath)]
    settings: Option<PathBuf>,

    /// Add system media controls for player interaction
    /// with media keys and your operating system.
    #[clap(long, action)]
    media_controls: bool,
}

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);

fn read_settings(path: Option<PathBuf>) -> Result<Settings, ConfigError> {
    #[cfg(debug_assertions)]
    if path.is_none() {
        return Ok(Settings::default());
    }

    let Some(file) = path.or_else(|| get_config_dir().map(|dir| dir.join("settings.toml"))) else {
        return Err("Failed to get a default settings file path from the operating system")?;
    };

    if file.exists() {
        return Settings::read(file);
    }

    let parent_dir = file.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent_dir).map_err(|err| {
        ConfigError::new(
            format!("Failed to create directory path for '{}'.", file.display()),
            Some(ConfigErrorSource::Io(err)),
        )
    })?;

    let settings = Settings::new(file);
    settings.save()?;

    Ok(settings)
}

fn get_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from(
        bevystov::APP_QUALIFIER,
        bevystov::APP_ORGANIZATION,
        bevystov::APP_NAME,
    )
    .map(|dirs| dirs.config_dir().to_path_buf())
}
