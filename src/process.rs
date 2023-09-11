use std::{
    cell::Cell,
    ffi::OsStr,
    fmt,
    path::Path,
    process::{Command, ExitStatus, Output},
    str,
};

use anyhow::{Context as _, Result};

use crate::term;

macro_rules! cmd {
    ($program:expr $(, $arg:expr)* $(,)?) => {{
        let mut _cmd = $crate::process::ProcessBuilder::new($program);
        $(
            _cmd.arg($arg);
        )*
        _cmd
    }};
}

// A builder for an external process, inspired by https://github.com/rust-lang/cargo/blob/0.47.0/src/cargo/util/process_builder.rs
#[must_use]
pub(crate) struct ProcessBuilder {
    cmd: Command,
    /// `true` to include full program path in display.
    display_program_path: Cell<bool>,
}

impl ProcessBuilder {
    /// Creates a new `ProcessBuilder`.
    pub(crate) fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            cmd: Command::new(program.as_ref()),
            display_program_path: Cell::new(term::verbose()),
        }
    }

    /// Adds an argument to pass to the program.
    pub(crate) fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.cmd.arg(arg.as_ref());
        self
    }

    /// Adds multiple arguments to pass to the program.
    pub(crate) fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self {
        self.cmd.args(args);
        self
    }

    /// Enables all display-related flags.
    fn display_all(&mut self) {
        self.display_program_path.set(true);
    }

    /// Executes a process, waiting for completion, and mapping non-zero exit
    /// status to an error.
    pub(crate) fn run(&mut self) -> Result<()> {
        let status = self.cmd.status().with_context(|| {
            self.display_all();
            ProcessError::new(&format!("could not execute process {self}"), None, None)
        })?;
        if status.success() {
            Ok(())
        } else {
            self.display_all();
            Err(ProcessError::new(
                &format!("process didn't exit successfully: {self}"),
                Some(status),
                None,
            )
            .into())
        }
    }

    /// Executes a process, captures its stdio output, returning the captured
    /// output, or an error if non-zero exit status.
    pub(crate) fn run_with_output(&mut self) -> Result<Output> {
        let output = self.cmd.output().with_context(|| {
            self.display_all();
            ProcessError::new(&format!("could not execute process {self}"), None, None)
        })?;
        if output.status.success() {
            Ok(output)
        } else {
            self.display_all();
            Err(ProcessError::new(
                &format!("process didn't exit successfully: {self}"),
                Some(output.status),
                Some(&output),
            )
            .into())
        }
    }

    /// Executes a process, captures its stdio output, returning the captured
    /// standard output as a `String`.
    pub(crate) fn read(&mut self) -> Result<String> {
        let mut output = String::from_utf8(self.run_with_output()?.stdout).with_context(|| {
            self.display_all();
            format!("failed to parse output from {self}")
        })?;
        while output.ends_with('\n') || output.ends_with('\r') {
            output.pop();
        }
        Ok(output)
    }
}

// Based on https://github.com/rust-lang/cargo/blob/0.47.0/src/cargo/util/process_builder.rs
impl fmt::Display for ProcessBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            write!(f, "`")?;
        }

        if self.display_program_path.get() {
            write!(f, "{}", self.cmd.get_program().to_string_lossy())?;
        } else {
            write!(
                f,
                "{}",
                Path::new(self.cmd.get_program()).file_stem().unwrap().to_string_lossy()
            )?;
        }

        for arg in self.cmd.get_args() {
            write!(f, " {}", arg.to_string_lossy())?;
        }

        if !f.alternate() {
            write!(f, "`")?;
        }

        Ok(())
    }
}

// Based on https://github.com/rust-lang/cargo/blob/0.47.0/src/cargo/util/errors.rs
#[derive(Debug)]
struct ProcessError {
    /// A detailed description to show to the user why the process failed.
    desc: String,
}

impl ProcessError {
    /// Creates a new process error.
    ///
    /// `status` can be `None` if the process did not launch.
    /// `output` can be `None` if the process did not launch, or output was not captured.
    fn new(msg: &str, status: Option<ExitStatus>, output: Option<&Output>) -> Self {
        let exit = match status {
            Some(s) => s.to_string(),
            None => "never executed".to_string(),
        };
        let mut desc = format!("{msg} ({exit})");

        if let Some(out) = output {
            match str::from_utf8(&out.stdout) {
                Ok(s) if !s.trim().is_empty() => {
                    desc.push_str("\n--- stdout\n");
                    desc.push_str(s);
                }
                Ok(_) | Err(_) => {}
            }
            match str::from_utf8(&out.stderr) {
                Ok(s) if !s.trim().is_empty() => {
                    desc.push_str("\n--- stderr\n");
                    desc.push_str(s);
                }
                Ok(_) | Err(_) => {}
            }
        }

        Self { desc }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.desc, f)
    }
}

impl std::error::Error for ProcessError {}
