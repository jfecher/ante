use goldentests::{TestConfig, TestResult};

/// Shared by each test so the stdlib is only checked once.
fn stdlib_cache() -> String {
    // goldentests splits arguments like a shell, so use forward slashes to keep Windows paths intact.
    format!("--stdlib-cache {}/goldentests-stdlib.inc", env!("CARGO_TARGET_TMPDIR").replace('\\', "/"))
}

/// Run every test in the examples directory and assert against its expected output in the same file
#[test]
fn goldentests() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples", "// ");
    config.base_args = stdlib_cache();
    config.run_tests()
}

/// Test each codegen example with the C backend as well to ensure consistency
#[test]
fn goldentests_c_backend() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples/codegen", "// ");
    config.base_args = format!("--backend=c {}", stdlib_cache());
    config.run_tests()
}

/// Unoptimized builds existentialize generics by default, so test monomorphization separately
#[test]
fn goldentests_monomorphization() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples/codegen", "// ");
    config.base_args = format!("--generics=mono {}", stdlib_cache());
    config.run_tests()
}
