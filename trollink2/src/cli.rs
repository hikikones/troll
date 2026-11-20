use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(version, about, styles = CLAP_STYLING)]
pub struct Args {
    /// Optional path for your database file. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "FILE.db", value_hint = clap::ValueHint::FilePath)]
    pub database: Option<PathBuf>,

    /// Optional path for your assets directory. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    pub assets: Option<PathBuf>,

    /// Optional path for your settings file. By default,
    /// the location will be determined by the conventions of your operating system.
    #[arg(long, value_name = "FILE.toml", value_hint = clap::ValueHint::FilePath)]
    pub settings: Option<PathBuf>,
}

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
