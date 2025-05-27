use write_a_shell::parsing::Element::*;
pub use write_a_shell::parsing::{Cmd, Element, Parser};

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
				binary: "echo".to_string(),
				args: vec!["‘hello’".to_string()]
			}),
			Element::And,
			ElementCmd(Cmd {
				binary: "echo".to_string(),
				args: vec!["‘world’".to_string()]
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
				binary: "echo".to_string(),
				args: vec!["1".to_string()]
			})],
			[ElementCmd(Cmd {
				binary: "echo".to_string(),
				args: vec!["2".to_string()]
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
				binary: "true".to_string(),
				args: vec![]
			}),
			And,
			ElementCmd(Cmd {
				binary: "echo".to_string(),
				args: vec!["\"output\"".to_string()]
			})
		]
	);
	let parsed_false: Vec<Element> = Parser::new("false && echo \"output\"").parse().unwrap();
	assert_eq!(
		parsed_false,
		[
			ElementCmd(Cmd {
				binary: "false".to_string(),
				args: vec![]
			}),
			And,
			ElementCmd(Cmd {
				binary: "echo".to_string(),
				args: vec!["\"output\"".to_string()]
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
				binary: "true".to_string(),
				args: vec![]
			}),
			Or,
			ElementCmd(Cmd {
				binary: "echo".to_string(),
				args: vec!["\"output\"".to_string()]
			})
		]
	);
	let parsed_false: Vec<Element> = Parser::new("false || echo \"output\"").parse().unwrap();
	assert_eq!(
		parsed_false,
		[
			ElementCmd(Cmd {
				binary: "false".to_string(),
				args: vec![]
			}),
			Or,
			ElementCmd(Cmd {
				binary: "echo".to_string(),
				args: vec!["\"output\"".to_string()]
			})
		]
	);
}
