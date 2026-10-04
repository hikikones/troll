use bevystov::{
    app::App,
    database::Database,
    jukebox::{AudioPlayer, Jukebox},
};
use terminal::Terminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();

    let terminal = Terminal::new()?;

    let jukebox = Jukebox::new(AudioPlayer::new()?);
    let database = Database::new(args.dir);
    let mut app = App::new(database, jukebox);

    terminal.enter(|term| app.run(term))
}

#[derive(Debug, clap::Parser)]
#[command(version, about, styles = CLAP_STYLING)]
struct Args {
    /// The directory for your music.
    #[arg(value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    dir: std::path::PathBuf,
}

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
