pub mod app;
pub mod cli;
pub mod database;
pub mod modals;
pub mod pages;
pub mod settings;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_QUALIFIER: &str = "org";
pub const APP_ORGANIZATION: &str = "hikikones";

pub type AppDir = Result<std::path::PathBuf, &'static str>;
pub type AppDirRef<'a> = Result<&'a std::path::Path, &'a &'static str>;

pub fn get_app_dir() -> AppDir {
    directories::ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or("Unable to locate a standard application directory from the operating system.")
}
