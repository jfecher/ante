use goldentests::{TestConfig, TestResult};

// Run every test in the examples directory and assert against its expected output in the same file
#[test]
fn goldentests() -> TestResult<()> {
    let config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples", "// ");
    config.run_tests()
}

// Test each codegen example with the C backend as well to ensure consistency
#[test]
fn c_backend() -> TestResult<()> {
    let mut config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples/codegen", "// ");
    config.base_args = "--backend=c".to_string();
    config.run_tests()
}
