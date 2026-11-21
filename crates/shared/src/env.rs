pub type AppDir = Result<std::path::PathBuf, &'static str>;
pub type AppDirRef<'a> = Result<&'a std::path::Path, &'a &'static str>;

pub fn get_app_dir(qualifier: &str, organization: &str, name: &str) -> AppDir {
    directories::ProjectDirs::from(qualifier, organization, name)
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or("Unable to locate a standard application directory from the operating system.")
}
