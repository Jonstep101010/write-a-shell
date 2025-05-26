use write_a_shell::execution::ElementVec;
pub use write_a_shell::parsing::{Cmd, Element, Parser};

#[test]
fn integration_cmds_are_parsed() {
	let parsed: Vec<Vec<Element>> = "echo ‘hello’ && echo ‘world’"
		.split(';')
		.filter_map(|s| Parser::new(s).parse())
		.collect();
	assert_eq!(
		parsed,
		[[
			Element::ElementCmd(Cmd {
				binary: "echo".to_string(),
				args: vec!["‘hello’".to_string()]
			}),
			Element::And,
			Element::ElementCmd(Cmd {
				binary: "echo".to_string(),
				args: vec!["‘world’".to_string()]
			})
		]]
	);
	for elem in parsed {
		// check that we can call run
		elem.run();
	}
}
