use Element::ElementCmd;
use std::{
	fs::{File, OpenOptions},
	io::Write,
	process::{Child, Output, Stdio},
};

use crate::parsing::Element;

pub trait ElementVec {
	// Add your methods here
	fn run(self) -> Option<Output>;
	fn process_elements(
		self,
		previous_output: &mut Option<Output>,
		children: &mut Vec<Result<Child, std::io::Error>>,
	) -> Option<()>;
}

impl<'a> ElementVec for Vec<Element<'a>> {
	fn run(self) -> Option<Output> {
		let mut previous_output = None;
		let mut children: Vec<std::io::Result<Child>> = Vec::new();

		self.process_elements(&mut previous_output, &mut children)?;
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
	fn process_elements(
		self,
		previous_output: &mut Option<Output>,
		children: &mut Vec<Result<Child, std::io::Error>>,
	) -> Option<()> {
		let mut prev_reader: Option<Stdio> = None;
		let mut cmd_idx = 0;
		let piped_commands = self
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
			.collect::<Vec<_>>();
		for (idx, elem) in self.iter().enumerate() {
			match elem {
				ElementCmd(cmd) => {
					// Determine stdin
					let mut stdin_info = Stdio::inherit();
					let mut heredoc_content: Option<String> = None;
					let mut stdin_redirected = false;

					// Look backwards for any dangling redirections (e.g., "< file | cmd")
					// Continue past pipes until we find either a redirect or another command
					'backwards_dangling_redirs: for i in (0..idx).rev() {
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
										break 'backwards_dangling_redirs;
									}
								}
								break 'backwards_dangling_redirs;
							}
							Some(Element::Heredoc(_, content)) => {
								stdin_info = Stdio::piped();
								heredoc_content = Some(content.clone());
								stdin_redirected = true;
								break 'backwards_dangling_redirs;
							}
							Some(ElementCmd(_)) => break 'backwards_dangling_redirs,
							_ => continue 'backwards_dangling_redirs, /* loop past pipes to handle redirs */
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

					if is_piped && idx > 0 && self.get(idx - 1) == Some(&Element::Pipe) {
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
					*previous_output = match children.pop() {
						Some(Err(e)) => {
							eprintln!("{e:?}");
							None
						}
						Some(Ok(child)) => child.wait_with_output().ok(),
						None => previous_output.take(),
					};
					let status = previous_output.as_ref()?.status;
					if !status.success() {
						break;
					}
				}
				_ => continue,
			}
		}
		Some(())
	}
}
