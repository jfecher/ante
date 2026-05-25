use goldentests::{TestConfig, TestResult};

#[test]
fn goldentests() -> TestResult<()> {
    let config = TestConfig::new(env!("CARGO_BIN_EXE_ante"), "examples", "// ");
    config.run_tests()
}
