use goldentests::{ TestConfig, TestResult };

#[test]
fn goldentests() -> TestResult<()> {
    let mut config = TestConfig::new("target/debug/ante", "examples", "// ")?;
    config.verbose = true;
    config.run_tests()
}
