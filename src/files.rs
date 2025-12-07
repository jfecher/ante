use std::{fs::File, io::{Read, Write}, path::{Path, PathBuf}};

use crate::incremental::Db;

/// Deserialize the compiler from our metadata file, returning it along with the file.
///
/// If we fail, just default to a fresh compiler with no cached compilations.
pub fn make_compiler(source_files: &[PathBuf], incremental: bool) -> (Db, Option<PathBuf>) {
    let (mut compiler, metadata_file) = if let Some(file) = source_files.first() {
        let metadata_file = file.with_extension("inc");

        if incremental && let Ok(text) = read_file(&metadata_file) {
            (ron::from_str(&text).unwrap_or_default(), Some(metadata_file))
        } else {
            (Db::default(), None)
        }
    } else {
        (Db::default(), None)
    };

    // TODO: If the compiler is created from incremental metadata, any previous input
    // files that are no longer used are never cleared.
    crate::find_files::populate_crates_and_files(&mut compiler, source_files);
    (compiler, metadata_file)
}

fn write_file(file_name: &Path, text: &str) -> Result<(), String> {
    let mut metadata_file = File::create(file_name)
        .map_err(|error| format!("Failed to create file `{}`:\n{error}", file_name.display()))?;

    let text = text.as_bytes();
    metadata_file
        .write_all(text)
        .map_err(|error| format!("Failed to write to file `{}`:\n{error}", file_name.display()))
}

/// This could be changed so that we only write if the metadata actually
/// changed but to simplify things we just always write.
pub fn write_metadata(compiler: &Db, metadata_file: &Path) -> Result<(), String> {
    // Using `to_writer` here would avoid the intermediate step of creating the string
    let serialized = ron::to_string(compiler).map_err(|error| format!("Failed to serialize database:\n{error}"))?;
    write_file(metadata_file, &serialized)
}

pub(crate) fn read_file(file_name: &std::path::Path) -> Result<String, String> {
    let mut file =
        File::open(file_name).map_err(|error| format!("Failed to open `{}`:\n{error}", file_name.display()))?;

    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| format!("Failed to read from file `{}`:\n{error}", file_name.display()))?;

    Ok(text)
}
