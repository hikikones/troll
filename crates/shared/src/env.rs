pub type AppDir = Result<std::path::PathBuf, NoAppDir>;
pub type AppDirRef<'a> = Result<&'a std::path::Path, &'a NoAppDir>;

pub fn get_app_dir(qualifier: &str, organization: &str, name: &str) -> AppDir {
    directories::ProjectDirs::from(qualifier, organization, name)
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or(NoAppDir)
}

#[derive(Debug, Clone, Copy)]
pub struct NoAppDir;

impl std::fmt::Display for NoAppDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Unable to locate a standard application directory from the operating system.")
    }
}

impl std::error::Error for NoAppDir {}
