use std::{error::Error, fmt, io::Error as IoError, path::Path};

use crate::incremental::Crate;

pub const MANIFEST_FILE_NAME: &str = "ante.toml";

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Manifest {
    pub name: Option<String>,

    /// Native libraries to link the final binary against, e.g. `link-lib = ["raylib"]`.
    #[serde(rename = "link-lib")]
    pub link_lib: Option<Vec<String>>,

    /// Extra native library search directories, passed to the linker as `-L<path>`.
    #[serde(rename = "link-search")]
    pub link_search: Option<Vec<String>>,
}

#[derive(Debug)]
pub enum CreateManifestError {
    Io(IoError),
    AlreadyExists,
}

impl fmt::Display for CreateManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::AlreadyExists => write!(formatter, "Manifest File Already Exists At This Location"),
        }
    }
}

impl Error for CreateManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::AlreadyExists => None,
        }
    }
}

impl From<IoError> for CreateManifestError {
    fn from(error: IoError) -> Self {
        Self::Io(error)
    }
}

impl Manifest {
    /// Creates a manifest if it does not exist at the given path.
    ///
    /// Errors on IO problems or if the manifest already exists.
    pub fn init(root: &Path) -> Result<(), CreateManifestError> {
        let manifest_path = root.join(MANIFEST_FILE_NAME);

        if std::fs::exists(&manifest_path)? {
            return Err(CreateManifestError::AlreadyExists);
        }

        let name = root.canonicalize()?.file_name().map(|name| name.to_string_lossy().into_owned());
        let default_manifest = Manifest { name, ..Manifest::default() };
        let default_manifest_string =
            toml::to_string_pretty(&default_manifest).expect("Failed to serialise default manifest");

        Ok(std::fs::write(manifest_path, default_manifest_string)?)
    }

    /// Read and parse the `ante.toml` manifest in the given crate root directory, if present.
    pub fn read(root: &Path) -> Option<Self> {
        let contents = std::fs::read_to_string(root.join(MANIFEST_FILE_NAME)).ok()?;
        toml::from_str(&contents).ok()
    }

    /// Applies this manifest's name & link options onto a crate
    pub fn apply(self, crate_: &mut Crate) {
        if let Some(name) = self.name {
            crate_.name = name;
        }
        crate_.link_libs = self.link_lib.unwrap_or_default();
        crate_.link_search_paths = self.link_search.unwrap_or_default();
    }
}
