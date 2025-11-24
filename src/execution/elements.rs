use crate::parsing::{
	Cmd,
	Element::{self, ElementCmd},
};
use std::{
	fs::{File, OpenOptions},
	io::Write,
	ops::ControlFlow,
	process::{Child, Output, Stdio},
};

pub trait ElementVec {
	fn new_elementcmd<'a>(
		&self,
		children: &mut Vec<Result<Child, std::io::Error>>,
		prev_reader: &mut Option<Stdio>,
		cmd_idx: &mut usize,
		piped_commands: &[Cmd<'a>],
		idx: usize,
		cmd: &Cmd<'a>,
	) -> ControlFlow<()>;
	fn handle_dangling_redirs(
		&self,
		idx: usize,
		stdin_info: &mut Stdio,
		heredoc_content: &mut Option<String>,
		stdin_redirected: &mut bool,
	);
	fn run(self) -> Option<Output>;
	fn process_elements(
		self,
		previous_output: &mut Option<Output>,
		children: &mut Vec<Result<Child, std::io::Error>>,
	) -> Option<()>;
}

impl ElementVec for Vec<Element<'_>> {
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
		let piped_commands: Vec<Cmd<'_>> = self
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
		'outer: for (idx, elem) in self.iter().enumerate() {
			match elem {
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
				ElementCmd(cmd) => {
					if let ControlFlow::Break(_) = &self.new_elementcmd(
						children,
						&mut prev_reader,
						&mut cmd_idx,
						&piped_commands,
						idx,
						cmd,
					) {
						continue 'outer;
					}
				}
				_ => continue 'outer,
			}
		}
		Some(())
	}
	fn handle_dangling_redirs(
		&self,
		idx: usize,
		stdin_info: &mut Stdio,
		heredoc_content: &mut Option<String>,
		stdin_redirected: &mut bool,
	) {
		'inner: for i in (0..idx).rev() {
			match self.get(i) {
				Some(Element::RedirectIn(filename)) => {
					match File::open(filename) {
						Ok(file) => {
							*stdin_info = Stdio::from(file);
							*stdin_redirected = true;
						}
						Err(e) => {
							eprintln!("Error opening {}: {}", filename, e);
							break 'inner; // if open fails: skip element
						}
					}
					break 'inner;
				}
				Some(Element::Heredoc(_, content)) => {
					*stdin_info = Stdio::piped();
					*heredoc_content = Some(content.clone());
					*stdin_redirected = true;
					break;
				}
				Some(ElementCmd(_)) => break 'inner,
				_ => continue 'inner,
			}
		}
	}
	fn new_elementcmd<'a>(
		&self,
		children: &mut Vec<Result<Child, std::io::Error>>,
		prev_reader: &mut Option<Stdio>,
		cmd_idx: &mut usize,
		piped_commands: &[Cmd<'a>],
		idx: usize,
		cmd: &Cmd,
	) -> ControlFlow<()> {
		let mut stdin_info = Stdio::inherit();
		let mut heredoc_content: Option<String> = None;
		let mut stdin_redirected = false;
		self.handle_dangling_redirs(
			idx,
			&mut stdin_info,
			&mut heredoc_content,
			&mut stdin_redirected,
		);
		if !stdin_redirected && let Some(next_elem) = self.get(idx + 1) {
			match next_elem {
				Element::RedirectIn(filename) => match File::open(filename) {
					Ok(file) => {
						stdin_info = Stdio::from(file);
						stdin_redirected = true;
					}
					Err(e) => {
						eprintln!("Error opening {}: {}", filename, e);
						return ControlFlow::Break(());
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
		let is_piped = self.get(idx + 1) == Some(&Element::Pipe)
			|| (idx > 0 && self.get(idx - 1) == Some(&Element::Pipe));
		if !stdin_redirected && is_piped {
			stdin_info = if let Some(reader) = prev_reader.take() {
				reader
			} else {
				Stdio::inherit()
			};
		}
		let mut stdout_info = Stdio::inherit();
		match self.get(idx + 1) {
			None => {}
			Some(elem) => match elem {
				Element::RedirectOut(filename) => match File::create(filename) {
					Ok(file) => {
						stdout_info = Stdio::from(file);
					}
					Err(e) => {
						eprintln!("Error creating {}: {}", filename, e);
						return ControlFlow::Break(());
					}
				},
				Element::RedirectAppend(filename) => {
					match OpenOptions::new().append(true).create(true).open(filename) {
						Ok(file) => {
							stdout_info = Stdio::from(file);
						}
						Err(e) => {
							eprintln!("Error opening {} for append: {}", filename, e);
							return ControlFlow::Break(());
						}
					}
				}
				Element::Pipe => {
					if *cmd_idx < piped_commands.len() - 1
						&& let Ok((reader, writer)) = std::io::pipe()
					{
						*prev_reader = Some(reader.into());
						stdout_info = writer.into();
					}
				}
				_ => return ControlFlow::Break(()),
			},
		}
		let (external, builtin) = cmd.run(stdin_info, stdout_info, is_piped);
		if is_piped && idx > 0 && self.get(idx - 1) == Some(&Element::Pipe) {
			*cmd_idx += 1;
		}

		if let Some(non_builtin_external) = external {
			write_heredoc_content_stdin(children, heredoc_content, non_builtin_external);
		} else if let Some(Ok(Some(output_builtin))) = builtin {
			std::io::stdout().write_all(&output_builtin.stdout).unwrap();
		}
		ControlFlow::Continue(())
	}
}

fn write_heredoc_content_stdin(
	children: &mut Vec<Result<Child, std::io::Error>>,
	heredoc_content: Option<String>,
	non_builtin_external: Result<Child, std::io::Error>,
) {
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
}
