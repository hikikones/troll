use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(version, about, styles = CLAP_STYLING)]
pub struct Args {
    /// The directory for your music.
    #[arg(value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    pub dir: PathBuf,

    /// Optional path for your settings file.
    #[arg(long, value_name = "FILE.toml", value_hint = clap::ValueHint::FilePath)]
    pub settings: Option<PathBuf>,

    /// Add system media controls for player interaction with media keys.
    #[clap(long, action)]
    pub media_controls: bool,

    /// Store database in-memory.
    #[clap(long, action)]
    pub in_memory: bool,
}

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
