use std::{
	io,
	io::IsTerminal, // <--- bring is_terminal() into scope
	io::Write,      // <--- bring flush() into scope
};

use crate::execution::elements::ElementVec;
mod execution;
mod parsing;

fn main() -> io::Result<()> {
	let history = execution::builtins::History::default();
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
		history
			.add(input_line.trim())
			.expect("cannot open histfile");

		// Parse line into executable command
		let mut cmd_chains = parsing::parse_multiple(&input_line);

		// Handle heredocs: collect content until delimiter is seen
		for chain in &mut cmd_chains {
			for elem in chain.iter_mut() {
				if let parsing::Element::Heredoc(delimiter, content) = elem {
					// Read lines until we see the delimiter
					let mut heredoc_content = String::new();
					loop {
						let mut line = String::new();
						std::io::stdin().read_line(&mut line)?;
						if line.trim() == *delimiter {
							break;
						}
						heredoc_content.push_str(&line);
					}
					*content = heredoc_content;
				}
			}
		}
		for cmd in cmd_chains {
			// Execute the command in a separate process
			// @follow-up take output and print stout
			let output = ElementVec::run(cmd);
			if let Some(output) = output {
				std::io::stdout().write_all(&output.stdout).unwrap();
			}
		}
		// Show output
	}
}
