use std::process::{Command, Output};

use tempfile::tempdir;

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn init_creates_a_runnable_program() {
    let directory = tempdir().unwrap();
    let project = directory.path().join("hello-world");
    let ante = env!("CARGO_BIN_EXE_ante");

    let init = Command::new(ante).arg("init").arg(&project).output().unwrap();
    assert_success(&init);

    let run = Command::new(ante).args(["run", "--backend", "c"]).current_dir(&project).output().unwrap();
    assert_success(&run);
    assert_eq!(String::from_utf8_lossy(&run.stdout), "Hello, World!\n");
}
