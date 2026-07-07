use std::path::Path;

use crate::incremental::Crate;

#[derive(serde::Deserialize, Default)]
pub struct Manifest {
    pub name: Option<String>,

    /// Native libraries to link the final binary against, e.g. `link-lib = ["raylib"]`.
    #[serde(rename = "link-lib")]
    pub link_lib: Option<Vec<String>>,

    /// Extra native library search directories, passed to the linker as `-L<path>`.
    #[serde(rename = "link-search")]
    pub link_search: Option<Vec<String>>,
}

impl Manifest {
    /// Read and parse the `ante.toml` manifest in the given crate root directory, if present.
    pub fn read(root: &Path) -> Option<Manifest> {
        let contents = std::fs::read_to_string(root.join("ante.toml")).ok()?;
        toml::from_str(&contents).ok()
    }

    /// Copy a manifest's native-link settings onto a crate.
    pub fn apply(&self, crate_: &mut Crate) {
        crate_.link_libs = self.link_lib.clone().unwrap_or_default();
        crate_.link_search_paths = self.link_search.clone().unwrap_or_default();
    }
}
