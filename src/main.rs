use std::{
	io,
	io::IsTerminal, // <--- bring is_terminal() into scope
	io::Write,      // <--- bring flush() into scope
	process::Command,
};

// @follow-up try using &str instead
#[derive(PartialEq, Debug)]
struct Cmd {
	binary: String,
	args: Vec<String>,
}

#[derive(PartialEq, Debug)]
enum Element {
	/// `&&`
	And,
	/// `||`
	Or,
	/// Command.
	#[allow(clippy::enum_variant_names)]
	ElementCmd(Cmd),
}

trait ElementVec {
	// Add your methods here
	fn run(self);
}

impl ElementVec for Vec<Element> {
	// Implement your methods here
	fn run(self) {
		use Element::ElementCmd;
		let mut previous_output = None;
		for elem in self {
			match elem {
				ElementCmd(cmd) => {
					previous_output = cmd.run();
				}
				Element::And => {
					let status = previous_output.expect("no command before &&").status;
					if !status.success() {
						break;
					}
					previous_output = None;
				}
				Element::Or => {
					let status = previous_output.expect("no command before ||").status;
					if !status.success() {
						break;
					}
					previous_output = None;
				}
			}
		}
	}
}

/// processes input: `chain_in_line` (already split by ';')
/// Parse `[Element]`s from a string.
#[derive(Debug)]
struct Parser {
	current: usize,
	tokens: Vec<String>,
}

impl Parser {
	///
	/// `chain_in_line`: single input to split by whitespace
	fn new(chain_in_line: &str) -> Self {
		dbg!(Self {
			tokens: chain_in_line.split_whitespace().map(String::from).collect(),
			current: 0,
		})
	}
	fn parse_cmd(&mut self, binary: String) -> Option<Cmd> {
		let mut args: Vec<String> = vec![];
		loop {
			let next = self.tokens.get(self.current);
			match next {
				Some(token) if token == "&&" => break,
				Some(token) if token == "||" => break,
				Some(token) => {
					args.push(token.to_string());
				}
				None => break,
			}
			self.current += 1;
		}
		Some(dbg!(Cmd { binary, args }))
	}
	fn parse(mut self) -> Option<Vec<Element>> {
		let mut elements = vec![];
		while let Some(elem) = self
			.tokens
			.get(self.current)
			.map(|s| s.to_string())
			.and_then(|next| {
				self.current += 1;
				match next.as_str() {
					"&&" => Some(Element::And),
					"||" => Some(Element::Or),
					_ => Self::parse_cmd(&mut self, next.to_string()).map(Element::ElementCmd),
				}
			}) {
			elements.push(elem);
		}
		// handle empty
		if elements.is_empty() {
			None
		} else {
			Some(elements)
		}
	}
}

// get single input from stdin
// run single command
impl Cmd {
	// replaced by Parser: from_line (single cmd)
	/// runs command in separate process
	pub fn run(self) -> Option<std::process::Output> {
		let process = Command::new(self.binary)
			.args(self.args)
			.spawn()
			.map_err(|e| eprintln!("{e:?}"))
			.ok()?;
		process.wait_with_output().ok()
	}
}

fn parse_multiple(line: &str) -> Vec<Vec<Element>> {
	// inefficient: parses whole line with split (instead of char by char)
	line.split(';')
		.filter_map(|s| Parser::new(s).parse())
		.collect()
}

fn main() -> io::Result<()> {
	loop {
		// show prompt
		let mut stdout = std::io::stdout();
		if stdout.is_terminal() {
			write!(stdout, "> ")?;
			stdout.flush().expect("failed to flush stdout");
		}
		// Read line from standard input
		let mut input_line = String::new();
		std::io::stdin().read_line(&mut input_line)?;
		// Parse line into executable command
		for cmd in parse_multiple(&input_line) {
			// Execute the command in a separate process
			cmd.run()
		}
		// Show output
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use Element::*;

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
}
