pub use crate::parsing::{Cmd, Element};
use std::process::{Child, Command, Output, Stdio};

pub trait ElementVec {
	// Add your methods here
	fn run(self) -> Option<Output>;
}

impl ElementVec for Vec<Element> {
	// Implement your methods here
	fn run(self) -> Option<Output> {
		use Element::ElementCmd;
		let mut previous_output = None;
		let mut commands: Vec<Cmd> = Vec::new();
		for elem in self.iter() {
			if let Element::ElementCmd(cmd) = elem {
				commands.push(cmd.clone());
			}
		}
		let mut prev_reader: Option<Stdio> = None;
		let mut cmd_idx = 0;
		let mut children: Vec<std::io::Result<Child>> = Vec::new();
		for elem in self {
			match elem {
				Element::Pipe => continue,
				ElementCmd(_) => {
					let stdin_info = if let Some(reader) = prev_reader.take() {
						reader
					} else {
						// first
						Stdio::inherit()
					};
					let stdout_info = if let Some((reader, writer)) =
						(cmd_idx != commands.len() - 1).then_some(std::io::pipe().ok()?)
					{
						prev_reader = Some(reader.into());
						writer.into()
					} else {
						// last
						Stdio::inherit()
					};
					let command = commands.get(cmd_idx).unwrap();
					if let Some(non_builtin_external) = command.run(stdin_info, stdout_info) {
						children.push(non_builtin_external)
					};
					cmd_idx += 1;
				}
				Element::And => {
					while let Some(child_result) = children.pop() {
						previous_output = {
							if let Ok(child) = child_result {
								child.wait_with_output().ok()
							} else {
								let e =
									child_result.expect_err("violated precondition: not an Err()");
								eprintln!("{e:?}");
								None
							}
						};
					}
					let status = previous_output.as_ref()?.status;
					if !status.success() {
						break;
					}
				}
				Element::Or => {
					while let Some(child_result) = children.pop() {
						previous_output = {
							if let Ok(child) = child_result {
								child.wait_with_output().ok()
							} else {
								let e =
									child_result.expect_err("violated precondition: not an Err()");
								eprintln!("{e:?}");
								None
							}
						};
					}
					let status = previous_output.as_ref()?.status;
					if status.success() {
						break;
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
	/// Option: Some() denotes external
	pub fn run(&self, stdin_info: Stdio, stdout_info: Stdio) -> Option<std::io::Result<Child>> {
		// set up args for builtins
		// let result = match self.binary.as_ref() {
		// match &self.binary {
		// "cd" => {
		// 	let dir = self.args.first()?;
		// 	let dir_pbuf = PathBuf::from(dir);
		// 	builtins::Cd::new(dir_pbuf).run()
		// }
		// "pwd" => {
		// 	let path = std::env::current_dir().expect("cwd should exist!");
		// 	println!("{}", path.display());
		// 	Ok(None)
		// }
		// "exit" => {
		// 	let status = match self.args.first() {
		// 		Some(status) => status.parse().unwrap_or_default(),
		// 		None => 0,
		// 	};
		// 	builtins::Exit::new(status).run()
		// }
		// "history" => builtins::History::default().run(),
		// external_binary => Some(
		// Command::new(external_binary)
		// 	.args(self.args)
		// 	.stdin(stdin_info)
		// 	.stdout(stdout_info)
		// 	.spawn(),
		// ),
		// }
		Some(
			Command::new(&self.binary)
				.args(&self.args)
				.stdin(stdin_info)
				.stdout(stdout_info)
				.spawn(),
		)
		// };
		// match result {
		// 	Ok(opt_output) => {
		// 		if let Some(output) = &opt_output {
		// 			// @remind only print stderr (stdout goes to pipe)
		// 			std::io::stderr().write_all(&output.stderr).unwrap();
		// 		}
		// 		opt_output
		// 	}
		// 	Err(e) => {
		// 		eprintln!("{e:?}");
		// 		None
		// 	}
		// }
	}
	// @note this implementation is incorrect as it waits for previous process to finish
	// pub fn run_external(self, stdin_info: Stdio, stdout_info: Stdio) -> std::io::Result<Child> {
	// let mut process = Command::new(self.binary);
	// process.args(self.args);
	// // @audit-ok set up stdin to be pipe (provide previous_output)
	// if previous_output.is_some() {
	// 	process.stdin(Stdio::piped());
	// }
	// // @audit-ok set up pipe before spawn
	// let mut child = process
	// 	.stdout(Stdio::piped())
	// 	.stderr(Stdio::piped())
	// 	.spawn()?;
	// if let Some(output) = &previous_output {
	// 	if let Some(mut stdin) = child.stdin.take() {
	// 		stdin.write_all(&output.stdout)?;
	// 	}
	// }
	// // @remind write previous_output to child stdin
	// let output = child.wait_with_output()?;
	// Ok(Some(output))
	// }
}
