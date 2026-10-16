// use std::path::{Path, PathBuf};

// use bevystov::{
//     app::App,
//     database::Database,
//     jukebox::{AudioPlayer, Jukebox},
//     mpris::MediaControls,
//     settings::{ConfigError, ConfigErrorSource, Settings},
// };
// use terminal::{Rgb, Terminal};

// fn main() -> std::process::ExitCode {
//     let args: Args = clap::Parser::parse();

//     if let Err(err) = run(args) {
//         eprintln!("Error: {err}");

//         let mut source = err.source();
//         while let Some(cause) = source {
//             eprintln!("Caused by: {cause}");
//             source = cause.source();
//         }

//         return std::process::ExitCode::FAILURE;
//     }

//     std::process::ExitCode::SUCCESS
// }

// fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
//     let terminal = Terminal::stdout();

//     let jukebox = Jukebox::new(AudioPlayer::new()?);
//     let database = Database::new(args.dir);

//     let mut settings = read_settings(args.settings)?;
//     settings.colors_mut().background =
//         Rgb::query_bg(&mut terminal.lock(), &mut std::io::stdin().lock())?
//             .slight_offset()
//             .as_color();

//     let media_controls = if args.media_controls {
//         Some(MediaControls::spawn()?)
//     } else {
//         None
//     };

//     let mut app = App::new(database, jukebox, settings, media_controls);

//     terminal.enter(|term| app.run(term))
// }

// #[derive(Debug, clap::Parser)]
// #[command(version, about, styles = CLAP_STYLING)]
// struct Args {
//     /// The directory for your music.
//     #[arg(value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
//     dir: std::path::PathBuf,

//     /// Optional path for your settings file. By default,
//     /// the location will be determined by the conventions of your operating system.
//     #[arg(long, value_name = "FILE.toml", value_hint = clap::ValueHint::FilePath)]
//     settings: Option<PathBuf>,

//     /// Add system media controls for player interaction
//     /// with media keys and your operating system.
//     #[clap(long, action)]
//     media_controls: bool,
// }

// const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
//     .header(clap_cargo::style::HEADER)
//     .usage(clap_cargo::style::USAGE)
//     .literal(clap_cargo::style::LITERAL)
//     .placeholder(clap_cargo::style::PLACEHOLDER)
//     .error(clap_cargo::style::ERROR)
//     .valid(clap_cargo::style::VALID)
//     .invalid(clap_cargo::style::INVALID);

// fn read_settings(path: Option<PathBuf>) -> Result<Settings, ConfigError> {
//     #[cfg(debug_assertions)]
//     if path.is_none() {
//         return Ok(Settings::default());
//     }

//     let Some(file) = path.or_else(|| get_config_dir().map(|dir| dir.join("settings.toml"))) else {
//         return Err("Failed to get a default settings file path from the operating system")?;
//     };

//     if file.exists() {
//         return Settings::read(file);
//     }

//     let parent_dir = file.parent().unwrap_or(Path::new("."));
//     std::fs::create_dir_all(parent_dir).map_err(|err| {
//         ConfigError::new(
//             format!("Failed to create directory path for '{}'.", file.display()),
//             Some(ConfigErrorSource::Io(err)),
//         )
//     })?;

//     let settings = Settings::new(file);
//     settings.save()?;

//     Ok(settings)
// }

// fn get_config_dir() -> Option<PathBuf> {
//     directories::ProjectDirs::from(
//         bevystov::APP_QUALIFIER,
//         bevystov::APP_ORGANIZATION,
//         bevystov::APP_NAME,
//     )
//     .map(|dirs| dirs.config_dir().to_path_buf())
// }

use serde::{Deserialize, Serialize};
use shared::redb::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Redb::in_memory()?;

    db.insert(TrackId(0), Track::new("Track0"))?;

    Ok(())
}

#[derive(Debug)]
struct Track<'a> {
    title: &'a str,
}

impl<'a> Track<'a> {
    fn new(title: &'a str) -> Self {
        Self { title }
    }
}

#[derive(Debug, Clone, Copy)]
struct TrackId(u64);

struct Tracks;

impl RedbTable for Tracks {
    type Id = TrackId;
    type Value = TrackValue;
    const NAME: &str = "Tracks";
}

struct TrackValue;

impl RedbValue for TrackValue {
    type SelfType<'a> = Track<'a>;
    type AsBytes<'a> = &'a str;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        todo!()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        todo!()
    }

    fn type_name() -> RedbTypeName {
        todo!()
    }
}

impl RedbValue for TrackId {
    type SelfType<'a> = Self;
    type AsBytes<'a> = <u64 as RedbValue>::AsBytes<'a>;

    fn fixed_width() -> Option<usize> {
        <u64 as RedbValue>::fixed_width()
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        Self(<u64 as RedbValue>::from_bytes(data))
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        <u64 as RedbValue>::as_bytes(&value.0)
    }

    fn type_name() -> RedbTypeName {
        RedbTypeName::new("TrackId")
    }
}

impl RedbKey for TrackId {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        <u64 as RedbKey>::compare(data1, data2)
    }
}

struct AlbumId(u64);

struct Album<'a> {
    title: &'a str,
}
