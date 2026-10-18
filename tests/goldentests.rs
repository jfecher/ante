use goldentests::{TestConfig, TestResult};

/// Shared by each test so the stdlib is only checked once
const STDLIB_CACHE: &str = concat!("--stdlib-cache ", env!("CARGO_TARGET_TMPDIR"), "/goldentests-stdlib.inc");

/// Run every test in the examples directory and assert against its expected output in the same file
#[test]
fn goldentests() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples", "// ");
    config.base_args = STDLIB_CACHE.to_string();
    config.run_tests()
}

/// Test each codegen example with the C backend as well to ensure consistency
#[test]
fn goldentests_c_backend() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples/codegen", "// ");
    config.base_args = format!("--backend=c {STDLIB_CACHE}");
    config.run_tests()
}

/// Unoptimized builds existentialize generics by default, so test monomorphization separately
#[test]
fn goldentests_monomorphization() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples/codegen", "// ");
    config.base_args = format!("--generics=mono {STDLIB_CACHE}");
    config.run_tests()
}
