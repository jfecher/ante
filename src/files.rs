use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::incremental::Db;

/// Deserialize the compiler from our metadata file, returning it along with the file.
///
/// If we fail, just default to a fresh compiler with no cached compilations.
pub fn make_compiler(source_files: &[PathBuf], incremental: bool) -> (Db, Option<PathBuf>) {
    let (mut compiler, metadata_file) = if let Some(file) = source_files.first()
        && incremental
    {
        let metadata_file = file.with_extension("inc");
        let db = match read_binary_file(&metadata_file) {
            Ok(bytes) => rmp_serde::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Db::default(),
        };
        (db, Some(metadata_file))
    } else {
        (Db::default(), None)
    };

    // TODO: If the compiler is created from incremental metadata, any previous input
    // files that are no longer used are never cleared.
    let local_crate_root = std::env::current_dir().unwrap_or_default();
    crate::find_files::populate_crates_and_files(&mut compiler, &local_crate_root, source_files);
    (compiler, metadata_file)
}

/// This could be changed so that we only write if the metadata actually
/// changed but to simplify things we just always write.
pub fn write_metadata(compiler: &Db, metadata_file: &Path) -> Result<(), String> {
    let bytes = rmp_serde::to_vec(compiler).map_err(|error| format!("Failed to serialize database:\n{error}"))?;

    let mut file = File::create(metadata_file)
        .map_err(|error| format!("Failed to create file `{}`:\n{error}", metadata_file.display()))?;
    file.write_all(&bytes).map_err(|error| format!("Failed to write to file `{}`:\n{error}", metadata_file.display()))
}

pub(crate) fn read_file(file_name: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(file_name).map_err(|error| format!("Failed to read `{}`:\n{error}", file_name.display()))
}

fn read_binary_file(file_name: &std::path::Path) -> Result<Vec<u8>, String> {
    let mut file =
        File::open(file_name).map_err(|error| format!("Failed to open `{}`:\n{error}", file_name.display()))?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("Failed to read from file `{}`:\n{error}", file_name.display()))?;

    Ok(bytes)
}
