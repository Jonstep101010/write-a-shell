use std::{
	io,
	io::IsTerminal, // <--- bring is_terminal() into scope
	io::Write,      // <--- bring flush() into scope
};
mod execution;
mod parsing;

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
		for cmd in parsing::parse_multiple(&input_line) {
			// Execute the command in a separate process
			execution::ElementVec::run(cmd);
		}
		// Show output
	}
}
