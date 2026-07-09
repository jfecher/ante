use std::process::Command;

use colored::Colorize;

use crate::{find_files::find_nearest_project_root, manifest::MANIFEST_FILE_NAME};

pub const DEPENDENCIES_DIR_NAME: &str = "deps";

/// Add a git dependency to the current Ante project by cloning it into the dependencies directory.
pub fn add_git_dependency(dep_url: &str) {
    let Some(project_root) = find_nearest_project_root() else {
        eprintln!(
            "{}: could not find {MANIFEST_FILE_NAME} in the current directory or any parent directory",
            "error".red(),
        );
        std::process::exit(1);
    };

    let deps_dir = project_root.join(DEPENDENCIES_DIR_NAME);
    if let Err(error) = std::fs::create_dir_all(&deps_dir) {
        eprintln!("{}: failed to create `{}`:\n{error}", "error".red(), deps_dir.display());
        std::process::exit(1);
    }

    let status = Command::new("git").arg("clone").arg(dep_url).current_dir(&deps_dir).status();
    match status {
        Ok(status) if status.success() => (),
        Ok(status) => {
            let code = status.code().map_or_else(|| "terminated by signal".to_string(), |code| code.to_string());
            eprintln!("{}: git clone failed with exit status {code}", "error".red());
            std::process::exit(1);
        },
        Err(error) => {
            eprintln!("{}: failed to run git clone:\n{error}", "error".red());
            std::process::exit(1);
        },
    }
}
