#[derive(PartialEq, Debug, Clone)]
pub struct Cmd<'a> {
	pub binary: &'a str,
	pub args: Vec<&'a str>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Element<'a> {
	/// `|`
	Pipe,
	/// `&&`
	And,
	/// `||`
	Or,
	/// Command.
	#[allow(clippy::enum_variant_names)]
	ElementCmd(Cmd<'a>),
}

/// processes input: `chain_in_line` (already split by ';')
/// Parse `[Element]`s from a string.
#[derive(Debug)]
pub struct Parser<'a> {
	current: usize,
	tokens: Vec<&'a str>,
}

impl<'a> Parser<'a> {
	///
	/// `chain_in_line`: single input to split by whitespace
	pub fn new(chain_in_line: &'a str) -> Self {
		Self {
			tokens: chain_in_line.split_whitespace().collect(),
			current: 0,
		}
	}
	fn parse_cmd(&mut self, binary: &'a str) -> Option<Cmd<'a>> {
		let mut args: Vec<&'a str> = vec![];
		loop {
			let next = self.tokens.get(self.current);
			match next {
				Some(&"|") | Some(&"&&") | Some(&"||") => break,
				Some(&token) => {
					args.push(token);
				}
				None => break,
			}
			self.current += 1;
		}
		Some(Cmd { binary, args })
	}
	pub fn parse(mut self) -> Option<Vec<Element<'a>>> {
		let mut elements = vec![];
		while let Some(&token) = self.tokens.get(self.current) {
			self.current += 1;
			elements.push(match token {
				"|" => Element::Pipe,
				"&&" => Element::And,
				"||" => Element::Or,
				_ => match Self::parse_cmd(&mut self, token) {
					Some(cmd) => Element::ElementCmd(cmd),
					None => break,
				},
			});
		}
		// handle empty
		(!elements.is_empty()).then_some(elements)
	}
}

pub fn parse_multiple(line: &str) -> Vec<Vec<Element<'_>>> {
	// inefficient: parses whole line with split (instead of char by char)
	line.split(';')
		.filter_map(|s| Parser::new(s.trim()).parse())
		.collect()
}

#[cfg(test)]
mod tests {
	use super::Element::ElementCmd;
	use super::*;
	#[test]
	fn no_cmd_is_parsed_from_empty_line() {
		assert_eq!(parse_multiple(""), Vec::<Vec<Element>>::new());
	}

	#[test]
	fn cmd_with_no_args_is_parsed() {
		assert_eq!(
			parse_multiple("ls"),
			vec![vec![ElementCmd(Cmd {
				binary: "ls",
				args: vec![]
			}),],]
		);
	}

	#[test]
	fn cmd_with_args_is_parsed() {
		assert_eq!(
			parse_multiple("ls -l"),
			vec![vec![ElementCmd(Cmd {
				binary: "ls",
				args: vec!["-l"]
			})]]
		);
	}
	#[test]
	fn cmds_are_parsed() {
		assert_eq!(
			parse_multiple("ls; echo hello"),
			[
				[ElementCmd(Cmd {
					binary: "ls",
					args: vec![]
				})],
				[ElementCmd(Cmd {
					binary: "echo",
					args: vec!["hello"]
				})]
			]
		)
	}

	#[test]
	fn pipe_is_parsed() {
		assert_eq!(
			parse_multiple("ls | wc -l"),
			vec![vec![
				ElementCmd(Cmd {
					binary: "ls",
					args: vec![]
				}),
				Element::Pipe,
				ElementCmd(Cmd {
					binary: "wc",
					args: vec!["-l"]
				}),
			]]
		);
	}
}
