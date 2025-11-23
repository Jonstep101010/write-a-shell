use write_a_shell::parsing::Element::*;
pub use write_a_shell::parsing::{Cmd, Element, Parser};
use write_a_shell::execution::ElementVec;
use std::fs;
use std::path::Path;

#[test]
fn integration_cmds_are_parsed_and() {
	let parsed: Vec<Vec<Element>> = "echo ‘hello’ && echo ‘world’"
		.split(';')
		.filter_map(|s| Parser::new(s).parse())
		.collect();
	assert_eq!(
		parsed,
		[[
			ElementCmd(Cmd {
				binary: "echo",
				args: vec!["‘hello’"],
				redirect_target: None,
			}),
			Element::And,
			ElementCmd(Cmd {
				binary: "echo",
				args: vec!["‘world’"],
				redirect_target: None,
			})
		]]
	);
}

#[test]
fn integration_cmds_are_parsed_semicolon() {
	let parsed: Vec<Vec<Element>> = "echo 1; echo 2"
		.split(';')
		.filter_map(|s| Parser::new(s).parse())
		.collect();
	assert_eq!(
		parsed,
		[
			[ElementCmd(Cmd {
				binary: "echo",
				args: vec!["1"],
				redirect_target: None,
			})],
			[ElementCmd(Cmd {
				binary: "echo",
				args: vec!["2"],
				redirect_target: None,
			})]
		]
	);
}

#[test]
fn integration_cmds_and_exprs() {
	let parsed_true: Vec<Element> = Parser::new("true && echo \"output\"").parse().unwrap();
	assert_eq!(
		parsed_true,
		[
			ElementCmd(Cmd {
				binary: "true",
				args: vec![],
				redirect_target: None,
			}),
			And,
			ElementCmd(Cmd {
				binary: "echo",
				args: vec!["\"output\""],
				redirect_target: None,
			})
		]
	);
	let parsed_false: Vec<Element> = Parser::new("false && echo \"output\"").parse().unwrap();
	assert_eq!(
		parsed_false,
		[
			ElementCmd(Cmd {
				binary: "false",
				args: vec![],
				redirect_target: None,
			}),
			And,
			ElementCmd(Cmd {
				binary: "echo",
				args: vec!["\"output\""],
				redirect_target: None,
			})
		]
	);
}

#[test]
fn integration_cmds_or_exprs() {
	let parsed_true: Vec<Element> = Parser::new("true || echo \"output\"").parse().unwrap();
	assert_eq!(
		parsed_true,
		[
			ElementCmd(Cmd {
				binary: "true",
				args: vec![],
				redirect_target: None,
			}),
			Or,
			ElementCmd(Cmd {
				binary: "echo",
				args: vec!["\"output\""],
				redirect_target: None,
			})
		]
	);
	let parsed_false: Vec<Element> = Parser::new("false || echo \"output\"").parse().unwrap();
	assert_eq!(
		parsed_false,
		[
			ElementCmd(Cmd {
				binary: "false",
				args: vec![],
				redirect_target: None,
			}),
			Or,
			ElementCmd(Cmd {
				binary: "echo",
				args: vec!["\"output\""],
				redirect_target: None,
			})
		]
	);
}

#[test]
fn integration_cmds_piped_exprs() {
	let parsed_pipes: Vec<Element> = Parser::new("cat | cat | ls").parse().unwrap();
	assert_eq!(
		parsed_pipes,
		[
			ElementCmd(Cmd {
				binary: "cat",
				args: vec![],
				redirect_target: None,
			}),
			Pipe,
			ElementCmd(Cmd {
				binary: "cat",
				args: vec![],
				redirect_target: None,
			}),
			Pipe,
			ElementCmd(Cmd {
				binary: "ls",
				args: vec![],
				redirect_target: None,
			})
		]
	);
}

#[test]
fn integration_cmd_with_redirection_parsing() {
	let parsed: Vec<Element> = Parser::new("echo hello > outfile").parse().unwrap();
	assert_eq!(
		parsed,
		[ElementCmd(Cmd {
			binary: "echo",
			args: vec!["hello"],
			redirect_target: Some("outfile"),
		})]
	);
}

#[test]
fn integration_redirection_execution_creates_file() {
	let test_file = "test_redirect_1.txt";
	// Clean up if file exists from previous run
	let _ = fs::remove_file(test_file);

	// Execute: echo hello > test_redirect_1.txt
	let cmd = format!("echo hello > {}", test_file);
	let elements = Parser::new(&cmd).parse().unwrap();
	let _output = elements.run();

	// Verify file was created and has correct content
	assert!(Path::new(test_file).exists(), "Redirect file should be created");
	let contents = fs::read_to_string(test_file).expect("Should read redirect file");
	assert_eq!(contents.trim(), "hello", "File should contain 'hello'");

	// Clean up
	fs::remove_file(test_file).expect("Should clean up test file");
}

#[test]
fn integration_redirection_with_pipe_execution() {
	let test_file = "test_redirect_2.txt";
	// Clean up if file exists from previous run
	let _ = fs::remove_file(test_file);

	// Execute: echo hello > test_redirect_2.txt | cat test_redirect_2.txt
	// This tests that redirection works in a pipeline - echo writes to file,
	// then cat reads and outputs it (output goes to inherited stdout, not captured)
	let cmd = format!("echo hello > {} | cat {}", test_file, test_file);
	let elements = Parser::new(&cmd).parse().unwrap();
	let _output = elements.run();

	// Verify file was created with correct content
	assert!(Path::new(test_file).exists(), "Redirect file should be created");
	let contents = fs::read_to_string(test_file).expect("Should read redirect file");
	assert_eq!(contents.trim(), "hello", "File should contain 'hello'");

	// Clean up
	fs::remove_file(test_file).expect("Should clean up test file");
}

#[test]
fn integration_multiple_pipes_with_redirection() {
	let test_file = "test_redirect_3.txt";
	// Clean up if file exists from previous run
	let _ = fs::remove_file(test_file);

	// Execute: echo test > test_redirect_3.txt | cat test_redirect_3.txt | wc -l
	// Tests multiple pipes with redirection - echo redirects to file, cat reads it,
	// wc counts lines (output goes to inherited stdout, not captured)
	let cmd = format!("echo test > {} | cat {} | wc -l", test_file, test_file);
	let elements = Parser::new(&cmd).parse().unwrap();
	let _output = elements.run();

	// Verify file was created with correct content
	assert!(Path::new(test_file).exists(), "Redirect file should be created");
	let contents = fs::read_to_string(test_file).expect("Should read redirect file");
	assert_eq!(contents.trim(), "test", "File should contain 'test'");

	// Clean up
	fs::remove_file(test_file).expect("Should clean up test file");
}

#[test]
fn integration_redirection_in_middle_of_pipe() {
	let test_file = "test_redirect_4.txt";
	// Clean up if file exists from previous run
	let _ = fs::remove_file(test_file);

	// Execute: echo first | echo second > test_redirect_4.txt | cat test_redirect_4.txt
	// Tests redirection in the middle of a pipeline - second echo redirects to file,
	// then cat reads and outputs it
	let cmd = format!("echo first | echo second > {} | cat {}", test_file, test_file);
	let elements = Parser::new(&cmd).parse().unwrap();
	let _output = elements.run();

	// Verify file was created with "second"
	assert!(Path::new(test_file).exists(), "Redirect file should be created");
	let contents = fs::read_to_string(test_file).expect("Should read redirect file");
	assert_eq!(contents.trim(), "second", "File should contain 'second'");

	// Clean up
	fs::remove_file(test_file).expect("Should clean up test file");
}
