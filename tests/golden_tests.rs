use goldentests::TestConfig;
use std::error::Error;

#[test]
fn goldentests() -> Result<(), Box<dyn Error>> {
    let config = TestConfig::new("target/debug/ante", "examples", "// ");
    config.run_tests()
}
