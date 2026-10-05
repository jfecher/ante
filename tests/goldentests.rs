use goldentests::{TestConfig, TestResult};

/// Run every test in the examples directory and assert against its expected output in the same file
#[test]
fn goldentests() -> TestResult<()> {
    TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples", "// ").run_tests()
}

/// Test each codegen example with the C backend as well to ensure consistency
#[test]
fn goldentests_c_backend() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples/codegen", "// ");
    config.base_args = "--backend=c".to_string();
    config.run_tests()
}

/// Unoptimized builds existentialize generics by default, so test monomorphization separately
#[test]
fn goldentests_monomorphization() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples/codegen", "// ");
    config.base_args = "--generics=mono".to_string();
    config.run_tests()
}
