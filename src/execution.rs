pub use crate::parsing::{Cmd, Element};
use std::{
	path::PathBuf,
	process::{Command, Output},
};

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
					if status.success() {
						break;
					}
					previous_output = None;
				}
			}
		}
	}
}

mod builtins {
	use std::{path::PathBuf, process::Output};

	pub struct Cd {
		dir: PathBuf,
	}

	impl Cd {
		pub fn new(dir: PathBuf) -> Self {
			Self { dir }
		}
		pub fn run(self) -> Result<Option<Output>, std::io::Error> {
			if self.dir.exists() {
				// go there
				std::env::set_current_dir(self.dir)?;
				Ok(None)
			} else {
				Err(std::io::ErrorKind::NotADirectory.into())
			}
		}
	}
	pub struct Exit {
		status: i32,
	}

	impl Exit {
		pub fn new(status: i32) -> Self {
			Self { status }
		}
		pub fn run(self) -> Result<Option<Output>, std::io::Error> {
			std::process::exit(self.status);
		}
	}
}

// get single input from stdin
// run single command
impl Cmd {
	// replaced by Parser: from_line (single cmd)
	/// runs command in separate process
	pub fn run(self) -> Option<std::process::Output> {
		// set up args for builtins
		let result = match self.binary.as_ref() {
			"cd" => {
				let dir = self.args.first()?;
				let dir_pbuf = PathBuf::from(dir);
				builtins::Cd::new(dir_pbuf).run()
			}
			"pwd" => {
				let path = std::env::current_dir().expect("cwd should exist!");
				println!("{}", path.display());
				Ok(None)
			}
			"exit" => {
				let status = match self.args.first() {
					Some(status) => status.parse().unwrap_or_default(),
					None => 0,
				};
				builtins::Exit::new(status).run()
			}
			_ => self.run_external(),
		};
		if let Err(e) = result {
			eprintln!("{e:?}");
		};
		None
	}
	pub fn run_external(self) -> Result<Option<Output>, std::io::Error> {
		let process = Command::new(self.binary).args(self.args).spawn()?;
		let output = process.wait_with_output()?;
		Ok(Some(output))
	}
}
