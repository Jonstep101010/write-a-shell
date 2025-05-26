pub use crate::parsing::{Cmd, Element};
use std::process::Command;

pub trait ElementVec {
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
