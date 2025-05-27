// @follow-up try using &str instead
#[derive(PartialEq, Debug)]
pub struct Cmd {
	pub binary: String,
	pub args: Vec<String>,
}

#[derive(PartialEq, Debug)]
pub enum Element {
	/// `|`
	Pipe,
	/// `&&`
	And,
	/// `||`
	Or,
	/// Command.
	#[allow(clippy::enum_variant_names)]
	ElementCmd(Cmd),
}

/// processes input: `chain_in_line` (already split by ';')
/// Parse `[Element]`s from a string.
#[derive(Debug)]
pub struct Parser {
	current: usize,
	tokens: Vec<String>,
}

impl Parser {
	///
	/// `chain_in_line`: single input to split by whitespace
	pub fn new(chain_in_line: &str) -> Self {
		Self {
			tokens: chain_in_line.split_whitespace().map(String::from).collect(),
			current: 0,
		}
	}
	fn parse_cmd(&mut self, binary: String) -> Option<Cmd> {
		let mut args: Vec<String> = vec![];
		loop {
			let next = self.tokens.get(self.current);
			match next {
				Some(token) if token == "|" => break,
				Some(token) if token == "&&" => break,
				Some(token) if token == "||" => break,
				Some(token) => {
					args.push(token.to_string());
				}
				None => break,
			}
			self.current += 1;
		}
		Some(Cmd { binary, args })
	}
	pub fn parse(mut self) -> Option<Vec<Element>> {
		let mut elements = vec![];
		while let Some(elem) = self
			.tokens
			.get(self.current)
			.map(|s| s.to_string())
			.and_then(|next| {
				self.current += 1;
				match next.as_str() {
					"|" => Some(Element::Pipe),
					"&&" => Some(Element::And),
					"||" => Some(Element::Or),
					_ => Self::parse_cmd(&mut self, next.to_string()).map(Element::ElementCmd),
				}
			}) {
			elements.push(elem);
		}
		// handle empty
		(!elements.is_empty()).then_some(elements)
	}
}

pub fn parse_multiple(line: &str) -> Vec<Vec<Element>> {
	// inefficient: parses whole line with split (instead of char by char)
	line.split(';')
		.filter_map(|s| Parser::new(s).parse())
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
				binary: "ls".to_string(),
				args: vec![]
			}),],]
		);
	}

	#[test]
	fn cmd_with_args_is_parsed() {
		assert_eq!(
			parse_multiple("ls -l"),
			vec![vec![ElementCmd(Cmd {
				binary: "ls".to_string(),
				args: vec!["-l".to_string()]
			})]]
		);
	}
	#[test]
	fn cmds_are_parsed() {
		assert_eq!(
			parse_multiple("ls; echo hello"),
			[
				[ElementCmd(Cmd {
					binary: "ls".to_string(),
					args: vec![]
				})],
				[ElementCmd(Cmd {
					binary: "echo".to_string(),
					args: vec!["hello".to_string()]
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
					binary: "ls".to_string(),
					args: vec![]
				}),
				Element::Pipe,
				ElementCmd(Cmd {
					binary: "wc".to_string(),
					args: vec!["-l".to_string()]
				}),
			]]
		);
	}
}
