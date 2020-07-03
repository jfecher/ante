use std::path::PathBuf;

pub struct TestConfig {
    /// The binary path to your program, typically "target/debug/myprogram"
    pub binary_path: String,

    /// The path to the subdirectory containing your tests. This subdirectory will be
    /// searched recursively for all files.
    pub test_path: PathBuf,

    /// The sequence of characters starting at the beginning of a line that
    /// all test options should be prefixed with. This is typically a comment
    /// in your language. For example, if we had a C like language we could
    /// have "// " as the test_line_prefix to allow "expected stdout:" and friends
    /// to be read inside comments at the start of a line.
    pub test_line_prefix: String,

    /// The "args:" keyword used while parsing tests. Anything after
    /// `test_line_prefix + test_args_prefix` is read in as a space-delimited
    /// argument to the program.
    pub test_args_prefix: String,

    /// The "expected stdout:" keyword used while parsing tests. Any line starting
    /// with `test_line_prefix` after a line starting with `test_line_prefix + test_stdout_prefix`
    /// is appended to the expected stdout output. This continues until the first
    /// line that does not start with `test_line_prefix`
    ///
    /// Example with `test_line_prefix = "// "` and `test_stdout_prefix = "expected stdout:"`
    /// ```
    /// // expected stdout:
    /// // first line of stdout
    /// // second line of stdout
    ///
    /// // Normal comment, expected stdout is done being read.
    /// ```
    pub test_stdout_prefix: String,

    /// The "expected stderr:" keyword used while parsing tests. Any line starting
    /// with `test_line_prefix` after a line starting with `test_line_prefix + test_stderr_prefix`
    /// is appended to the expected stderr output. This continues until the first
    /// line that does not start with `test_line_prefix`
    ///
    /// Example with `test_line_prefix = "-- "` and `test_stderr_prefix = "expected stderr:"`
    /// ```
    /// -- expected stderr:
    /// -- first line of stderr
    /// -- second line of stderr
    ///
    /// -- Normal comment, expected stderr is done being read.
    /// ```
    pub test_stderr_prefix: String,

    /// The "expected exit status:" keyword used while parsing tests. This will expect an
    /// integer after this keyword representing the expected exit status of the given test.
    ///
    /// Example with `test_line_prefix = "; "` and `test_exit_status_prefix = "expected exit status:"`
    /// ```
    /// // expected exit status: 0
    /// ```
    pub test_exit_status_prefix: String,
}

impl TestConfig {
    /// Creates a new TestConfig for the given binary path, test path, and prefix.
    ///
    /// If we were testing a C++-like language that uses `//` as its comment syntax, we
    /// may want our test keywords embedded in comments. Additionally, lets say our
    /// project is called "my-compiler" and our test path is "examples/goldentests".
    /// In that case we can construct a `TestConfig` like so:
    ///
    /// ```
    /// let config = TestConfig::new("target/debug/my-compiler", "examples/goldentests", "// ");
    /// ```
    ///
    /// This will give us the default keywords when parsing our test files which allows
    /// us to write tests such as the following:
    ///
    /// ```c++
    /// std::cout << "Hello, World!\n";
    /// std::cerr << "Goodbye, World!\n";
    ///
    /// // These are args to your program, so this:
    /// // args: --run
    /// // Gets translated to:  target/debug/my-compiler --run testfile
    ///
    /// // The expected exit status is optional, by default it is not checked.
    /// // expected exit status: 0
    ///
    /// // The expected stdout output however is mandatory. If it is omitted, it
    /// // is assumed that stdout should be empty after invoking the program.
    /// // expected stdout:
    /// // Hello, World!
    ///
    /// // The expected stderr output is also mandatory. If it is omitted it is
    /// // likewise assumed stderr should be empty.
    /// // expected stderr:
    /// // Goodbye, World!
    /// ```
    ///
    /// Note that we can still embed normal comments in the program even though our test
    /// line prefix was "// "! Any test line that doesn't start with a keyword like "args:"
    /// or "expected stdout:" is ignored unless it is following an "expected stdout:" or
    /// "expected stderr:", in which case it is appended to the expected output.
    ///
    /// If you want to change these default keywords you can also create a TestConfig
    /// via `TestConfig::with_custom_keywords` which will allow you to specify each.
    pub fn new(binary_path: &str, test_path: &str, test_line_prefix: &str) -> TestConfig {
        TestConfig::with_custom_keywords(
            binary_path,
            test_path,
            test_line_prefix,
            "args:",
            "expected stdout:",
            "expected stderr:",
            "expected exit status:"
        )
    }

    /// This function is provided in case you want to change the default keywords used when
    /// searching through the test file. This will let you change "expected stdout:"
    /// or any other keyword to "output I want ->" or any other arbitrary string so long as it
    /// does not contain "\n".
    ///
    /// If you don't want to change any of the defaults, you can use `TestConfig::new` to construct
    /// a TestConfig with the default keywords (which are listed in its documentation).
    pub fn with_custom_keywords(binary_path: &str, test_path: &str, test_line_prefix: &str,
                                test_args_prefix: &str, test_stdout_prefix: &str,
                                test_stderr_prefix: &str, test_exit_status_prefix: &str) -> TestConfig {
        TestConfig {
            binary_path:             binary_path.to_string(),
            test_path:               PathBuf::from(test_path),
            test_line_prefix:        test_line_prefix.to_string(),
            test_args_prefix:        test_line_prefix.to_string() + test_args_prefix,
            test_stdout_prefix:      test_line_prefix.to_string() + test_stdout_prefix,
            test_stderr_prefix:      test_line_prefix.to_string() + test_stderr_prefix,
            test_exit_status_prefix: test_line_prefix.to_string() + test_exit_status_prefix,
        }
    }
}
