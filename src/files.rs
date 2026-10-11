use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::SystemTime,
};

use crate::incremental::Db;

/// The incremental metadata file along with the state it was loaded in
pub struct MetadataFile {
    path: PathBuf,

    /// The compiler version loaded, if any
    loaded_version: Option<u32>,

    /// Written before the database to identify what it is valid for
    stamp: &'static [u8],
}

/// Deserialize the compiler from our metadata file, returning it along with the file.
///
/// If we fail, just default to a fresh compiler with no cached compilations.
pub fn make_compiler(
    source_files: &[PathBuf], incremental: bool, stdlib_cache: Option<&Path>,
) -> (Db, Option<MetadataFile>) {
    let (mut compiler, metadata_file) = if let Some(path) = stdlib_cache {
        match read_db(path, stdlib_cache_stamp()) {
            Some(db) => (db, None),
            None => {
                let metadata_file =
                    MetadataFile { path: path.to_owned(), loaded_version: None, stamp: stdlib_cache_stamp() };
                (Db::default(), Some(metadata_file))
            },
        }
    } else if let Some(file) = source_files.first()
        && incremental
    {
        let path = file.with_extension("inc");
        let db = read_db(&path, build_stamp());
        let loaded_version = db.as_ref().map(|db| db.version());
        (db.unwrap_or_default(), Some(MetadataFile { path, loaded_version, stamp: build_stamp() }))
    } else {
        (Db::default(), None)
    };

    // TODO: If the compiler is created from incremental metadata, any previous input
    // files that are no longer used are never cleared.

    // Use the discovered project root when building a project.
    // Fall back to the current directory for explicit file compilation.
    let local_crate_root =
        crate::find_files::find_nearest_project_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    crate::find_files::populate_crates_and_files(&mut compiler, &local_crate_root, source_files);
    (compiler, metadata_file)
}

/// Deserialize the database in `path`, ignoring it if it was written for a different `stamp`
fn read_db(path: &Path, stamp: &[u8]) -> Option<Db> {
    let bytes = read_binary_file(path).ok()?;
    let bytes = bytes.strip_prefix(stamp)?;
    let db = crate::shared_arc::with_sharing(|| postcard::from_bytes::<Db>(bytes));
    db.inspect_err(|error| eprintln!("warning: failed to load incremental cache `{}`: {error}", path.display())).ok()
}

/// Write the compiler's cache to the metadata file, unless it is unchanged.
pub fn write_metadata(compiler: &Db, metadata_file: &MetadataFile) -> Result<(), String> {
    if metadata_file.loaded_version == Some(compiler.version()) && !crate::incremental::query_ran() {
        return Ok(());
    }

    // Written to a temporary file first so an interrupted write never leaves a truncated cache.
    let stamp = metadata_file.stamp;
    let metadata_file = &metadata_file.path;
    let temporary = metadata_file.with_extension(format!("inc.tmp{}", std::process::id()));
    let write_error = |error| format!("Failed to write to file `{}`:\n{error}", temporary.display());

    let file = File::create(&temporary).map_err(write_error)?;
    let mut writer = BufWriter::with_capacity(1 << 20, file);
    writer.write_all(stamp).map_err(write_error)?;
    crate::shared_arc::with_sharing(|| postcard::to_io(compiler, &mut writer))
        .map_err(|error| format!("Failed to serialize database:\n{error}"))?;
    writer.flush().map_err(write_error)?;

    std::fs::rename(&temporary, metadata_file)
        .map_err(|error| format!("Failed to replace `{}`:\n{error}", metadata_file.display()))
}

/// Prefix of each metadata file identifying the compiler build that wrote it
fn build_stamp() -> &'static [u8] {
    static STAMP: OnceLock<Vec<u8>> = OnceLock::new();
    STAMP.get_or_init(|| {
        let executable = std::env::current_exe().and_then(std::fs::metadata);
        let (size, modified) = executable.map(|file| (file.len(), file.modified().ok())).unwrap_or_default();
        format!("ante {} {size} {modified:?}\n", env!("CARGO_PKG_VERSION")).into_bytes()
    })
}

/// [build_stamp] plus a fingerprint of the stdlib's source files, so editing the stdlib replaces a stale stdlib cache
fn stdlib_cache_stamp() -> &'static [u8] {
    static STAMP: OnceLock<Vec<u8>> = OnceLock::new();
    STAMP.get_or_init(|| {
        let mut files = Vec::new();
        collect_file_metadata(&crate::paths::stdlib_path().join(crate::project::SRC_FOLDER), &mut files);
        files.sort();
        let mut stamp = build_stamp().to_vec();
        stamp.extend_from_slice(format!("stdlib {:x}\n", crate::parser::ids::hash(&files)).as_bytes());
        stamp
    })
}

/// Collect the path, length, and modification time of every file in `dir`, recursively
fn collect_file_metadata(dir: &Path, files: &mut Vec<(PathBuf, u64, Option<SystemTime>)>) {
    let Ok(entries) = dir.read_dir() else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            collect_file_metadata(&path, files);
        } else {
            files.push((path, metadata.len(), metadata.modified().ok()));
        }
    }
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
