use std::{error::Error, fmt, io::Error as IoError, path::PathBuf, process::Command};

use crate::find_files::find_nearest_project_root;

pub const DEPENDENCIES_DIR_NAME: &str = "deps";

#[derive(Debug)]
pub enum DependencyError {
    MissingProjectRoot,
    Io(IoError),
    GitClone(String),
}

impl fmt::Display for DependencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProjectRoot => write!(formatter, "Missing project root; please create one with `ante init`"),
            Self::Io(error) => error.fmt(formatter),
            Self::GitClone(error) => write!(formatter, "Git clone failed: {error}"),
        }
    }
}

impl Error for DependencyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::MissingProjectRoot | Self::GitClone(_) => None,
        }
    }
}

impl From<IoError> for DependencyError {
    fn from(error: IoError) -> Self {
        Self::Io(error)
    }
}

/// Add a git dependency to the current Ante project by cloning it into the dependencies directory.
pub fn add_dependency(dep_url: &str) -> Result<(), DependencyError> {
    let project_root = find_nearest_project_root().ok_or(DependencyError::MissingProjectRoot)?;
    add_git_dependency(&project_root, dep_url)
}

/// Add a git dependency to the current Ante project by cloning it into the dependencies directory.
fn add_git_dependency(project_root: &PathBuf, dep_url: &str) -> Result<(), DependencyError> {
    let deps_dir = project_root.join(DEPENDENCIES_DIR_NAME);

    std::fs::create_dir_all(&deps_dir)?;

    let status = Command::new("git").arg("clone").arg(dep_url).current_dir(&deps_dir).status()?;

    match status {
        status if status.success() => Ok(()),
        status => {
            let code = status.code().map_or_else(|| "terminated by signal".to_string(), |code| code.to_string());
            Err(DependencyError::GitClone(code))
        },
    }
}
