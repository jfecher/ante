use crate::manifest::{CreateManifestError, MANIFEST_FILE_NAME, Manifest};
use std::{
    error::Error,
    fmt,
    io::Error as IoError,
    path::{Path, PathBuf},
};

/// Default name of the src folder.
pub(crate) const SRC_FOLDER: &str = "src";
pub(crate) const MAIN_FILE: &str = "main.an";
const DEFAULT_MAIN_SOURCE: &str = r#"main () =
    println "Hello, World!"
"#;

#[derive(Debug)]
pub enum InitProjectError {
    Manifest(CreateManifestError),
    Io(IoError),
    InvalidMainFile(PathBuf),
}

impl fmt::Display for InitProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::InvalidMainFile(path) => {
                write!(
                    formatter,
                    "Cannot create main source file because `{}` exists and is not a file",
                    path.display()
                )
            },
        }
    }
}

impl Error for InitProjectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Manifest(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::InvalidMainFile(_) => None,
        }
    }
}

impl From<CreateManifestError> for InitProjectError {
    fn from(error: CreateManifestError) -> Self {
        Self::Manifest(error)
    }
}

impl From<IoError> for InitProjectError {
    fn from(error: IoError) -> Self {
        Self::Io(error)
    }
}

pub fn init_project(path: &Path) -> Result<(), InitProjectError> {
    std::fs::create_dir_all(path)?;

    if std::fs::exists(path.join(MANIFEST_FILE_NAME))? {
        return Err(CreateManifestError::AlreadyExists.into());
    }

    let source_dir = path.join(SRC_FOLDER);
    std::fs::create_dir_all(&source_dir)?;

    let main_file = source_dir.join(MAIN_FILE);
    if main_file.exists() && !main_file.is_file() {
        return Err(InitProjectError::InvalidMainFile(main_file));
    } else if !main_file.exists() {
        std::fs::write(main_file, DEFAULT_MAIN_SOURCE)?
    }

    Manifest::init(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn initializes_project() {
        let directory = tempdir().unwrap();

        init_project(directory.path()).unwrap();

        assert!(directory.path().join("ante.toml").is_file());

        let main_file = directory.path().join(SRC_FOLDER).join(MAIN_FILE);
        assert!(main_file.is_file());
        assert_eq!(fs::read_to_string(main_file).unwrap(), DEFAULT_MAIN_SOURCE);
    }

    #[test]
    fn initializes_project_in_missing_directory() {
        let directory = tempdir().unwrap();
        let project_directory = directory.path().join("hello-world");

        init_project(&project_directory).unwrap();

        assert!(project_directory.join("ante.toml").is_file());
        assert!(project_directory.join(SRC_FOLDER).join(MAIN_FILE).is_file());
        assert_eq!(Manifest::read(&project_directory).unwrap().name.as_deref(), Some("hello-world"));
    }

    #[test]
    fn rejects_existing_project() {
        let directory = tempdir().unwrap();

        init_project(directory.path()).unwrap();
        let error = init_project(directory.path()).unwrap_err();

        assert!(matches!(error, InitProjectError::Manifest(CreateManifestError::AlreadyExists)));
    }

    #[test]
    fn preserves_existing_main_file() {
        let directory = tempdir().unwrap();
        let source_directory = directory.path().join(SRC_FOLDER);
        fs::create_dir(&source_directory).unwrap();

        let main_file = source_directory.join(MAIN_FILE);
        let existing_source = "main () = print \"Existing\"\n";
        fs::write(&main_file, existing_source).unwrap();

        init_project(directory.path()).unwrap();

        assert_eq!(fs::read_to_string(main_file).unwrap(), existing_source);
    }

    #[test]
    fn leaves_no_manifest_when_source_directory_creation_fails() {
        let directory = tempdir().unwrap();
        let source_directory = directory.path().join(SRC_FOLDER);
        fs::write(&source_directory, "not a directory").unwrap();

        init_project(directory.path()).unwrap_err();

        assert!(!directory.path().join(MANIFEST_FILE_NAME).exists());

        fs::remove_file(source_directory).unwrap();
        init_project(directory.path()).unwrap();
        assert!(directory.path().join(MANIFEST_FILE_NAME).is_file());
    }

    #[test]
    fn rejects_main_path_that_is_not_a_file() {
        let directory = tempdir().unwrap();
        let main_file = directory.path().join(SRC_FOLDER).join(MAIN_FILE);
        fs::create_dir_all(&main_file).unwrap();

        let error = init_project(directory.path()).unwrap_err();

        assert!(matches!(error, InitProjectError::InvalidMainFile(path) if path == main_file));
        assert!(!directory.path().join(MANIFEST_FILE_NAME).exists());
    }
}
