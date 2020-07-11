use goldentests::{ TestConfig, TestResult };

#[test]
fn goldentests() -> TestResult<()> {
    let config = TestConfig::new("target/debug/ante", "examples", "// ")?;
    config.run_tests()
}
