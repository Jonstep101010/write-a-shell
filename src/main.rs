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

// get single input from stdin
// run single command
impl Cmd {
	fn from_line(line: &str) -> Option<Self> {
		dbg!(&line);
		let mut parts = line.split_whitespace().map(String::from);
		parts.next().map(|binary| {
			dbg!(Cmd {
				binary,
				args: parts.collect(),
			})
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
		if let Some(cmd) = Cmd::from_line(&input_line) {
			// Execute the command in a separate process
			cmd.run()
		}
		// Show output
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn no_cmd_is_parsed_from_empty_line() {
		assert_eq!(Cmd::from_line(""), None);
	}

	#[test]
	fn cmd_with_no_args_is_parsed() {
		assert_eq!(
			Cmd::from_line("ls"),
			Some(Cmd {
				binary: "ls".to_string(),
				args: vec![]
			})
		);
	}

	#[test]
	fn cmd_with_args_is_parsed() {
		assert_eq!(
			Cmd::from_line("ls -l"),
			Some(Cmd {
				binary: "ls".to_string(),
				args: vec!["-l".to_string()]
			})
		);
	}
}
