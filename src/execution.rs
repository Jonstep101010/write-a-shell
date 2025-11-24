pub use crate::parsing::{Cmd, Element};
use std::{
	fs::{File, OpenOptions},
	io::Write,
	path::PathBuf,
	process::{Child, Command, Output, Stdio},
};

pub trait ElementVec {
	// Add your methods here
	fn run(self) -> Option<Output>;
}

impl<'a> ElementVec for Vec<Element<'a>> {
	fn run(self) -> Option<Output> {
		use Element::ElementCmd;
		let mut previous_output = None;
		let piped_commands: Vec<Cmd> = self
			.iter()
			.enumerate()
			.flat_map(|(idx, elem)| {
				if let Element::ElementCmd(cmd) = elem
					&& (self.get(idx + 1) == Some(&Element::Pipe)
						|| (idx > 0 && self.get(idx - 1) == Some(&Element::Pipe)))
				{
					return Some(cmd.clone());
				}
				None
			})
			.collect();
		let mut prev_reader: Option<Stdio> = None;
		let mut cmd_idx = 0;
		let mut children: Vec<std::io::Result<Child>> = Vec::new();

		for (idx, elem) in self.iter().enumerate() {
			match elem {
				ElementCmd(cmd) => {
					// Determine stdin
					let mut stdin_info = Stdio::inherit();
					let mut heredoc_content: Option<String> = None;
					let mut stdin_redirected = false;

					// Look backwards for any dangling redirections (e.g., "< file | cmd")
					// Continue past pipes until we find either a redirect or another command
					for i in (0..idx).rev() {
						match self.get(i) {
							Some(Element::RedirectIn(filename)) => {
								match File::open(filename) {
									Ok(file) => {
										// Convert to Stdio using raw fd, then forget the File
										// to transfer ownership to the Stdio/child process
										// let fd = file.as_raw_fd();
										// std::mem::forget(file);
										// stdin_info = unsafe { Stdio::from_raw_fd(fd) };
										stdin_info = Stdio::from(file);
										stdin_redirected = true;
									}
									Err(e) => {
										eprintln!("Error opening {}: {}", filename, e);
										// Continue to next element if we can't open the file
										break;
									}
								}
								break;
							}
							Some(Element::Heredoc(_, content)) => {
								stdin_info = Stdio::piped();
								heredoc_content = Some(content.clone());
								stdin_redirected = true;
								break;
							}
							Some(ElementCmd(_)) => break,
							_ => continue, /* loop past pipes to handle redirs */
						}
					}

					// Then check after (for syntax like: cat < file)
					if !stdin_redirected && let Some(next_elem) = self.get(idx + 1) {
						match next_elem {
							Element::RedirectIn(filename) => match File::open(filename) {
								Ok(file) => {
									stdin_info = Stdio::from(file);
									stdin_redirected = true;
								}
								Err(e) => {
									eprintln!("Error opening {}: {}", filename, e);
									continue;
								}
							},
							Element::Heredoc(_, content) => {
								// For heredoc, we need to write content to stdin
								stdin_info = Stdio::piped();
								heredoc_content = Some(content.clone());
								stdin_redirected = true;
							}
							_ => {}
						}
					}

					// Check if this command is part of a pipe
					let is_piped = self.get(idx + 1) == Some(&Element::Pipe)
						|| (idx > 0 && self.get(idx - 1) == Some(&Element::Pipe));

					// If stdin wasn't set by redirection and we're in a pipe, use pipe stdin
					if !stdin_redirected && is_piped {
						stdin_info = if let Some(reader) = prev_reader.take() {
							reader
						} else {
							Stdio::inherit()
						};
					}

					// Determine stdout
					let mut stdout_info = Stdio::inherit();

					// Check for output redirection after this command
					if let Some(next_elem) = self.get(idx + 1) {
						match next_elem {
							Element::RedirectOut(filename) => match File::create(filename) {
								Ok(file) => {
									stdout_info = Stdio::from(file);
								}
								Err(e) => {
									eprintln!("Error creating {}: {}", filename, e);
									continue;
								}
							},
							Element::RedirectAppend(filename) => {
								match OpenOptions::new().append(true).create(true).open(filename) {
									Ok(file) => {
										stdout_info = Stdio::from(file);
									}
									Err(e) => {
										eprintln!("Error opening {} for append: {}", filename, e);
										continue;
									}
								}
							}
							Element::Pipe => {
								// Only set up pipe if stdout wasn't redirected
								if cmd_idx < piped_commands.len() - 1
									&& let Ok((reader, writer)) = std::io::pipe()
								{
									prev_reader = Some(reader.into());
									stdout_info = writer.into();
								}
							}
							_ => {}
						}
					}

					let (external, builtin) = cmd.run(stdin_info, stdout_info, is_piped);

					if is_piped && idx > 0 && matches!(self.get(idx - 1), Some(Element::Pipe)) {
						cmd_idx += 1;
					}

					if let Some(non_builtin_external) = external {
						// If we have heredoc content, write it to stdin
						if let Some(content) = heredoc_content {
							if let Ok(mut child) = non_builtin_external {
								if let Some(mut stdin) = child.stdin.take() {
									use std::io::Write;
									let _ = stdin.write_all(content.as_bytes());
									drop(stdin); // Close stdin to signal EOF
								}
								children.push(Ok(child));
							} else {
								children.push(non_builtin_external);
							}
						} else {
							children.push(non_builtin_external);
						}
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
				_ => continue,
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
