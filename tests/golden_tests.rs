#![feature(str_strip)]

mod error;
mod config;
mod diff_printer;

use colored::Colorize;
use difference::Changeset;

use diff_printer::DiffPrinter;
use config::TestConfig;

use std::fs::File;
use std::path::{ Path, PathBuf };
use std::io::Read;
use std::error::Error;
use std::process::{ Command, Output };

type TestResult<T> = Result<T, Box<dyn Error>>;

struct Test {
    path: PathBuf,
    command_line_args: String,
    expected_stdout: String,
    expected_stderr: String,
    expected_exit_status: Option<i32>,
}

#[derive(PartialEq)]
enum TestParseState {
    Neutral,
    ReadingExpectedStdout,
    ReadingExpectedStderr,
}

fn find_tests(directory: &Path) -> TestResult<Vec<PathBuf>> {
    let mut tests = vec![];
    if directory.is_dir() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                tests.append(&mut find_tests(&path)?);
            } else {
                tests.push(path);
            }
        }
    }
    Ok(tests)
}

fn parse_test(test_path: &PathBuf, config: &TestConfig) -> TestResult<Test> {
    let path = test_path.clone();
    let mut command_line_args = String::new();
    let mut expected_stdout = String::new();
    let mut expected_stderr = String::new();
    let mut expected_exit_status = None;

    let mut file = File::open(test_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut state = TestParseState::Neutral;
    for line in contents.lines() {
        if line.starts_with(&config.test_line_prefix) {
            // If we're currently reading stdout or stderr, append the line to the expected output
            if state == TestParseState::ReadingExpectedStdout {
                expected_stdout += line.strip_prefix(&config.test_line_prefix).unwrap();
                expected_stdout += "\n";
            } else if state == TestParseState::ReadingExpectedStderr {
                expected_stderr += line.strip_prefix(&config.test_line_prefix).unwrap();
                expected_stderr += "\n";

            // Otherwise, look to see if the line begins with a keyword and if so change state
            // (stdout/stderr) or parse an argument to the keyword (args/exit status).

            // args:
            } else if line.starts_with(&config.test_args_prefix) {
                command_line_args = line.strip_prefix(&config.test_args_prefix).unwrap().to_string();

            // expected stdout:
            } else if line.starts_with(&config.test_stdout_prefix) {
                state = TestParseState::ReadingExpectedStdout;
                // Append the remainder of the line to the expected stdout.
                // Both expected_stdout and expected_stderr are trimmed so extra spaces if this is
                // empty shouldn't matter.
                expected_stdout += &(line.strip_prefix(&config.test_stdout_prefix).unwrap().to_string() + " ");

            // expected stderr:
            } else if line.starts_with(&config.test_stderr_prefix) {
                state = TestParseState::ReadingExpectedStderr;
                expected_stderr += &(line.strip_prefix(&config.test_stderr_prefix).unwrap().to_string() + " ");

            // expected exit status:
            } else if line.starts_with(&config.test_exit_status_prefix) {
                let status = line.strip_prefix(&config.test_stderr_prefix).unwrap().trim();
                expected_exit_status = Some(status.parse()?);
            }
        } else {
            state = TestParseState::Neutral;
        }
    }

    Ok(Test { path, command_line_args, expected_stdout, expected_stderr, expected_exit_status })
}

/// Diff the given "stream" and expected contents of the stream.
/// Returns non-zero on error.
fn check_for_differences_in_stream(path: &Path, name: &str, stream: &[u8], expected: &str) -> i8 {
    let output_string = String::from_utf8_lossy(stream);
    let output = output_string.trim();
    let expected = expected.trim();

    let differences = Changeset::new(expected, output, "\n");
    let distance = differences.distance;
    if distance != 0 {
        println!("{}: Actual {} differs from expected {}:\n{}\n",
                path.display().to_string().bright_yellow(), name, name, DiffPrinter(differences));
        1
    } else {
        0
    }
}

fn check_for_differences(output: &Output, test: &Test) -> bool {
    let mut error_count = 0;
    if let Some(expected_status) = test.expected_exit_status {
        if let Some(actual_status) = output.status.code() {
            if expected_status != actual_status {
                error_count += 1;
                println!("{}: Expected an exit status of {} but process returned {}\n",
                       test.path.display().to_string().bright_yellow(), expected_status, actual_status);
            }
        } else {
            error_count += 1;
            println!("{}: Expected an exit status of {} but process was terminated by signal instead\n",
                    test.path.display().to_string().bright_yellow(), expected_status);
        }
    }

    error_count += check_for_differences_in_stream(&test.path, "stdout", &output.stdout, &test.expected_stdout);
    error_count += check_for_differences_in_stream(&test.path, "stderr", &output.stderr, &test.expected_stderr);
    error_count != 0
}

fn run_tests(config: &TestConfig) -> TestResult<()> {
    let files = find_tests(&config.test_path)?;
    let tests = files.iter()
        .map(|file| parse_test(file, config))
        .collect::<Vec<_>>();

    let mut failing_tests = 0;
    for test in tests {
        let test = test?;

        let mut args = test.command_line_args.trim()
            .split(" ")
            .map(|s| s.to_owned())
            .collect::<Vec<_>>();

        args.push(test.path.to_string_lossy().to_string());

        let command = Command::new(&config.binary_path).args(args).output()?;
        let new_error = check_for_differences(&command, &test);
        if new_error {
            failing_tests += 1;
        }
    }

    if failing_tests != 0 {
        println!("{} {} tests are failing\n", failing_tests.to_string().red(), "golden".bright_yellow());
        Err(Box::new(error::TestError::ExpectedOutputDiffers))
    } else {
        Ok(())
    }
}

#[test]
fn goldentests() -> Result<(), Box<dyn Error>> {
    let config = TestConfig::new("target/debug/ante", "examples", "// ");
    run_tests(&config)
}
