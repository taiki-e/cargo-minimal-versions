// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    cell::Cell,
    ffi::OsStr,
    fmt,
    path::Path,
    process::{Command, ExitStatus, Output},
    str,
};

use anyhow::{Context as _, Error, Result};

use crate::term;

macro_rules! cmd {
    ($program:expr $(, $arg:expr)* $(,)?) => {{
        let mut _cmd = std::process::Command::new($program);
        $(
            _cmd.arg($arg);
        )*
        $crate::process::ProcessBuilder::from_std(_cmd)
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
    pub(crate) fn from_std(cmd: Command) -> Self {
        Self { cmd, display_program_path: Cell::new(term::verbose()) }
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

    /// Set a variable in the process's environment.
    pub(crate) fn env(&mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> &mut Self {
        self.cmd.env(key.as_ref(), val.as_ref());
        self
    }

    /// Enables all display-related flags.
    fn display_all(&self) {
        self.display_program_path.set(true);
    }

    /// Executes a process, waiting for completion, and mapping non-zero exit
    /// status to an error.
    pub(crate) fn run(&mut self) -> Result<()> {
        let status = self.cmd.status().with_context(|| {
            self.display_all();
            process_error(format!("could not execute process {self}"), None, None)
        })?;
        if status.success() {
            Ok(())
        } else {
            self.display_all();
            Err(process_error(
                format!("process didn't exit successfully: {self}"),
                Some(status),
                None,
            ))
        }
    }

    /// Executes a process, captures its stdio output, returning the captured
    /// output, or an error if non-zero exit status.
    pub(crate) fn run_with_output(&mut self) -> Result<Output> {
        let output = self.cmd.output().with_context(|| {
            self.display_all();
            process_error(format!("could not execute process {self}"), None, None)
        })?;
        if output.status.success() {
            Ok(output)
        } else {
            self.display_all();
            Err(process_error(
                format!("process didn't exit successfully: {self}"),
                Some(output.status),
                Some(&output),
            ))
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
            f.write_str("`")?;
        }

        if self.display_program_path.get() {
            f.write_str(&self.cmd.get_program().to_string_lossy())?;
        } else {
            f.write_str(&Path::new(self.cmd.get_program()).file_stem().unwrap().to_string_lossy())?;
        }

        for arg in self.cmd.get_args() {
            write!(f, " {}", arg.to_string_lossy())?;
        }

        if !f.alternate() {
            f.write_str("`")?;
        }

        Ok(())
    }
}

// Based on https://github.com/rust-lang/cargo/blob/0.47.0/src/cargo/util/errors.rs
/// Creates a new process error.
///
/// `status` can be `None` if the process did not launch.
/// `output` can be `None` if the process did not launch, or output was not captured.
fn process_error(mut msg: String, status: Option<ExitStatus>, output: Option<&Output>) -> Error {
    match status {
        Some(s) => {
            msg.push_str(" (");
            msg.push_str(&s.to_string());
            msg.push(')');
        }
        None => msg.push_str(" (never executed)"),
    }

    if let Some(out) = output {
        match str::from_utf8(&out.stdout) {
            Ok(s) if !s.trim().is_empty() => {
                msg.push_str("\n--- stdout\n");
                msg.push_str(s);
            }
            Ok(_) | Err(_) => {}
        }
        match str::from_utf8(&out.stderr) {
            Ok(s) if !s.trim().is_empty() => {
                msg.push_str("\n--- stderr\n");
                msg.push_str(s);
            }
            Ok(_) | Err(_) => {}
        }
    }

    Error::msg(msg)
}
