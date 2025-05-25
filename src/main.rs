use std::{
	io,
	io::IsTerminal, // <--- bring is_terminal() into scope
	io::Write,      // <--- bring flush() into scope
	process::Command,
};

// @follow-up try using &str instead
#[derive(PartialEq, Debug)]
struct CommandArgs {
	binary: String,
	args: Vec<String>,
}

// get single input from stdin
// run single command
impl CommandArgs {
	fn from_str(line: String) -> Option<Self> {
		dbg!(&line);
		let mut parts = line.split_whitespace().map(String::from);
		parts.next().map(|binary| {
			let check = CommandArgs {
				binary,
				args: parts.collect(),
			};
			dbg!(&check);
			check
		})
	}
	///
	/// runs command in separate process
	pub fn run(self) {
		let process = Command::new(self.binary).args(self.args).spawn();
		match process {
			Ok(mut child) => {
				child.wait().expect("command wasn't running");
			}
			Err(e) => eprintln!("{e:?}"),
		}
	}
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
		if let Some(cmd) = CommandArgs::from_str(input_line) {
			// Execute the command in a separate process
			cmd.run()
		}
		// Show output
	}
}
