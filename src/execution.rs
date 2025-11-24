pub use crate::parsing::Cmd;
use std::{
	path::PathBuf,
	process::{Child, Command, Output, Stdio},
};

pub mod elements;

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
		pub fn run(self) -> ! {
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

pub type ExternalWithChild = Option<std::io::Result<Child>>;
pub type BuiltinWithOutput = Option<Result<Option<Output>, std::io::Error>>;

// get single input from stdin
// run single command
impl<'a> Cmd<'a> {
	// replaced by Parser: from_line (single cmd)
	/// runs command in separate process
	/// Option: Some() denotes external
	pub fn run(
		&self,
		stdin_info: Stdio,
		stdout_info: Stdio,
		is_piped: bool,
	) -> (ExternalWithChild, BuiltinWithOutput) {
		// set up args for builtins
		match self.binary {
			"cd" => {
				let dir = self.args.first();
				if dir.is_none() {
					return (None, Some(Ok(None)));
				}
				let dir_pbuf = PathBuf::from(dir.unwrap());
				(None, Some(builtins::Cd::new(dir_pbuf).run()))
			}
			"pwd" => {
				let path = std::env::current_dir().expect("cwd to be valid!");

				(None, Some(Ok(Some(Output {
				status: <std::process::ExitStatus as std::os::unix::process::ExitStatusExt>::from_raw(0),
				stdout: [path.as_path().to_str().unwrap(), "\n"].concat().into_bytes(),
				stderr: Vec::new(),
		   		 }))))
			}
			"exit" => {
				let status = match self.args.first() {
					Some(status) => status.parse().unwrap_or_default(),
					None => 0,
				};
				// Only exit the shell if not piped
				if is_piped {
					// Return exit status as successful, but don't actually exit
					(None, Some(Ok(Some(Output {
						status: <std::process::ExitStatus as std::os::unix::process::ExitStatusExt>::from_raw(status),
						stdout: Vec::new(),
						stderr: Vec::new(),
					}))))
				} else {
					builtins::Exit::new(status).run();
				}
			}
			"history" => (None, Some(builtins::History::default().run())),
			_ => (
				Some(
					Command::new(self.binary)
						.args(&self.args)
						.stdin(stdin_info)
						.stdout(stdout_info)
						.spawn(),
				),
				None,
			),
		}
	}
}
