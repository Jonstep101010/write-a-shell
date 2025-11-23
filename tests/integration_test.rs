use write_a_shell::parsing::Element::*;
pub use write_a_shell::parsing::{Cmd, Element, Parser};

#[test]
fn integration_cmds_are_parsed_and() {
	let parsed: Vec<Vec<Element>> = "echo 'hello' && echo 'world'"
		.split(';')
		.filter_map(|s| Parser::new(s).parse())
		.collect();
	let expected: Vec<Vec<Element>> = vec![vec![
		ElementCmd(Cmd {
			binary: "echo",
			args: vec!["'hello'"],
			redirect_target: None,
		}),
		Element::And,
		ElementCmd(Cmd {
			binary: "echo",
			args: vec!["'world'"],
			redirect_target: None,
		})
	]];
	assert_eq!(parsed, expected);
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
fn integration_cmd_with_redirection() {
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
