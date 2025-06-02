pub use crate::parsing::{Cmd, Element};
use std::{
	io::Write,
	path::PathBuf,
	process::{Child, Command, Output, Stdio},
};

pub trait ElementVec {
	// Add your methods here
	fn run(self) -> Option<Output>;
}

impl ElementVec for Vec<Element> {
	// Implement your methods here
	fn run(self) -> Option<Output> {
		use Element::ElementCmd;
		let mut previous_output = None;
		let piped_commands: Vec<Cmd> = self
			.iter()
			.enumerate()
			.flat_map(|(idx, elem)| {
				if let Element::ElementCmd(cmd) = elem {
					if self.get(idx + 1) == Some(&Element::Pipe)
						|| (idx > 0 && self.get(idx - 1) == Some(&Element::Pipe))
					{
						return Some(cmd.clone());
					}
				}
				None
			})
			.collect();
		let mut prev_reader: Option<Stdio> = None;
		let mut cmd_idx = 0;
		let mut children: Vec<std::io::Result<Child>> = Vec::new();
		for (idx, elem) in self.iter().enumerate() {
			match elem {
				Element::Pipe => continue,
				ElementCmd(cmd) => {
					let (external, builtin) = if self.get(idx + 1) == Some(&Element::Pipe)
						|| (idx > 0 && self.get(idx - 1) == Some(&Element::Pipe))
					{
						let stdin_info = if let Some(reader) = prev_reader.take() {
							reader
						} else {
							// first
							Stdio::inherit()
						};
						let stdout_info = if let Some((reader, writer)) =
							(cmd_idx != piped_commands.len() - 1).then_some(std::io::pipe().ok()?)
						{
							prev_reader = Some(reader.into());
							writer.into()
						} else {
							// last
							Stdio::inherit()
						};
						let state = piped_commands
							.get(cmd_idx)
							.expect("pipe: precondition to hold")
							.run(stdin_info, stdout_info);
						cmd_idx += 1;
						state
					} else {
						// not in pipes
						cmd.run(Stdio::inherit(), Stdio::inherit())
					};
					if let Some(non_builtin_external) = external {
						children.push(non_builtin_external)
					} else if let Some(Ok(Some(output_builtin))) = builtin {
						std::io::stdout().write_all(&output_builtin.stdout).unwrap();
					}
				}
				Element::And | Element::Or => {
					previous_output = match children.pop() {
						Some(Err(e)) => {
							eprintln!("{e:?}");
							None
						}
						Some(Ok(child)) => child.wait_with_output().ok(),
						None => previous_output,
					};
					let status = previous_output.as_ref()?.status;
					match elem {
						Element::And => {
							if !status.success() {
								break;
							}
						}
						Element::Or => {
							if status.success() {
								break;
							}
						}
						_ => unreachable!("match excludes!"),
					}
				}
			}
		}
		for child_result in children {
			previous_output = {
				if let Ok(child) = child_result {
					child.wait_with_output().ok()
				} else {
					let e = child_result.expect_err("violated precondition: not an Err()");
					eprintln!("{e:?}");
					None
				}
			};
		}
		previous_output
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
impl Cmd {
	// replaced by Parser: from_line (single cmd)
	/// runs command in separate process
	/// Option: Some() denotes external
	pub fn run(
		&self,
		stdin_info: Stdio,
		stdout_info: Stdio,
	) -> (ExternalWithChild, BuiltinWithOutput) {
		// set up args for builtins
		match self.binary.as_ref() {
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
				builtins::Exit::new(status).run();
			}
			"history" => (None, Some(builtins::History::default().run())),
			_ => (
				Some(
					Command::new(&self.binary)
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
