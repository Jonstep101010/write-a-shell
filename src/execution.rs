pub use crate::parsing::{Cmd, Element};
use std::{
	io::Write,
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

pub mod builtins {
	use std::{
		os::unix::process::ExitStatusExt,
		path::PathBuf,
		process::{ExitStatus, Output},
	};

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

	pub struct History {
		history_file_path: PathBuf,
	}

	impl History {
		///
		/// assumes file exists with var
		fn new(history_file_path: PathBuf) -> Self {
			Self { history_file_path }
		}

		///
		/// display history
		pub fn run(self) -> Result<Option<Output>, std::io::Error> {
			// read history to memory
			let history = std::fs::read_to_string(self.history_file_path)?;
			// return output with history...
			Ok(Some(Output {
				status: ExitStatus::from_raw(0),
				stdout: history.into_bytes(),
				stderr: vec![],
			}))
		}

		///
		/// extend with `cmd`
		pub fn add(&self, cmd: &str) -> Result<(), std::io::Error> {
			// openflags: create, append
			use std::io::Write;
			let mut histfile = std::fs::OpenOptions::new()
				.append(true)
				.create(true)
				.open(&self.history_file_path)?;
			writeln!(histfile, "{cmd}")
		}
	}
	impl Default for History {
		fn default() -> Self {
			let fpath = std::env::var("HISTORY_PATH").unwrap_or(".history".to_string());
			History::new(PathBuf::from(fpath))
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
			"history" => builtins::History::default().run(),
			_ => self.run_external(),
		};
		if let Err(e) = result {
			eprintln!("{e:?}");
		} else if let Ok(Some(output)) = result {
			std::io::stdout().write_all(&output.stdout).unwrap();
			std::io::stderr().write_all(&output.stderr).unwrap();
		}
		None
	}
	pub fn run_external(self) -> Result<Option<Output>, std::io::Error> {
		let process = Command::new(self.binary).args(self.args).spawn()?;
		let output = process.wait_with_output()?;
		Ok(Some(output))
	}
}
