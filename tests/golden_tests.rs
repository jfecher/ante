use goldentests::{TestConfig, TestResult};

#[test]
fn goldentests() -> TestResult<()> {
    let config = TestConfig::new("target/debug/ante", "examples", "// ");
    config.run_tests()
}

/// Codegen tests are run with cranelift since no backend is explicitly specified.
/// We want to test all backends so re-run them here with `--backend=llvm`.
#[test]
fn llvm_codegen() -> TestResult<()> {
    let mut config = TestConfig::new("target/debug/ante", "examples/codegen", "// ");
    config.base_args = "--backend=llvm".to_owned();
    config.run_tests()
}
