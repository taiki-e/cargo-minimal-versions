// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{env, ffi::OsString};

use anyhow::{bail, format_err, Result};
use lexopt::{
    Arg::{Long, Short, Value},
    ValueExt,
};

use crate::term;

static USAGE: &str = "cargo-minimal-versions\n
Cargo subcommand for proper use of -Z minimal-versions.
\nUSAGE:
    cargo minimal-versions <CARGO_SUBCOMMAND> [OPTIONS] [CARGO_OPTIONS]
\nCARGO_SUBCOMMANDS:
    build
    check
    test
    ...
";

pub(crate) struct Args {
    pub(crate) no_private: bool,
    pub(crate) subcommand: Subcommand,
    pub(crate) manifest_path: Option<String>,
    pub(crate) cargo_args: Vec<String>,
    pub(crate) rest: Vec<String>,
}

#[derive(PartialEq)]
pub(crate) enum Subcommand {
    // build, check, run
    Builtin,
    // test, bench
    BuiltinDev,
    Other,
}

impl Subcommand {
    fn new(s: &str) -> Self {
        // https://github.com/rust-lang/cargo/blob/0.62.0/src/bin/cargo/main.rs#L48-L56
        match s {
            "b" | "build" | "c" | "check" | "r" | "run" => Self::Builtin,
            "t" | "test" | "bench" => Self::BuiltinDev,
            _ => {
                warn!("unrecognized subcommand '{s}'");
                Self::Other
            }
        }
    }

    pub(crate) fn always_needs_dev_deps(&self) -> bool {
        *self == Self::BuiltinDev
    }
}

impl Args {
    pub(crate) fn parse() -> Result<Self> {
        const SUBCMD: &str = "minimal-versions";

        // rustc/cargo args must be valid Unicode
        // https://github.com/rust-lang/rust/blob/1.70.0/compiler/rustc_driver_impl/src/lib.rs#L1366-L1376
        fn handle_args(
            args: impl IntoIterator<Item = impl Into<OsString>>,
        ) -> impl Iterator<Item = Result<String>> {
            args.into_iter().enumerate().map(|(i, arg)| {
                arg.into()
                    .into_string()
                    .map_err(|arg| format_err!("argument {} is not valid Unicode: {arg:?}", i + 1))
            })
        }

        let mut raw_args = handle_args(env::args_os());
        raw_args.next(); // cargo
        match raw_args.next().transpose()? {
            Some(a) if a == SUBCMD => {}
            Some(a) => bail!("expected subcommand '{SUBCMD}', found argument '{a}'"),
            None => bail!("expected subcommand '{SUBCMD}'"),
        }
        let mut args = vec![];
        for arg in &mut raw_args {
            let arg = arg?;
            if arg == "--" {
                break;
            }
            args.push(arg);
        }
        let rest = raw_args.collect::<Result<Vec<_>>>()?;

        let mut cargo_args = vec![];
        let mut subcommand = None;
        let mut color = None;
        let mut manifest_path: Option<String> = None;
        let mut verbose = 0;

        let mut no_private = false;

        let mut parser = lexopt::Parser::from_args(args);
        while let Some(arg) = parser.next()? {
            macro_rules! parse_opt {
                ($opt:ident $(,)?) => {{
                    if $opt.is_some() {
                        multi_arg(&arg)?;
                    }
                    $opt = Some(parser.value()?.parse()?);
                }};
            }
            macro_rules! parse_flag {
                ($flag:ident $(,)?) => {{
                    if $flag {
                        multi_arg(&arg)?;
                    }
                    $flag = true;
                }};
            }

            match arg {
                Long("color") => parse_opt!(color),
                Long("manifest-path") => parse_opt!(manifest_path),
                Short('v') | Long("verbose") => verbose += 1,

                // cargo-hack flags
                // However, do not propagate to cargo-hack, as the same process
                // is done by cargo-minimal-versions.
                Long("remove-dev-deps" | "no-dev-deps") => {} // TODO: warn?
                // Turn --ignore-private into --no-private.
                Long("ignore-private" | "no-private") => parse_flag!(no_private),

                Short('h') | Long("help") if subcommand.is_none() => {
                    print!("{USAGE}");
                    std::process::exit(0);
                }
                Short('V') | Long("version") if subcommand.is_none() => {
                    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }

                // passthrough
                Long(flag) => {
                    let flag = format!("--{flag}");
                    if let Some(val) = parser.optional_value() {
                        cargo_args.push(format!("{flag}={}", val.string()?));
                    } else {
                        cargo_args.push(flag);
                    }
                }
                Short(flag) => {
                    if matches!(flag, 'q' | 'r') {
                        // To handle combined short flags properly, handle known
                        // short flags without value as special cases.
                        cargo_args.push(format!("-{flag}"));
                    } else if let Some(val) = parser.optional_value() {
                        cargo_args.push(format!("-{flag}{}", val.string()?));
                    } else {
                        cargo_args.push(format!("-{flag}"));
                    }
                }
                Value(val) => {
                    let val = val.string()?;
                    if subcommand.is_none() {
                        subcommand = Some(Subcommand::new(&val));
                    }
                    cargo_args.push(val);
                }
            }
        }

        term::set_coloring(color)?;

        let Some(subcommand) = subcommand else { bail!("expected subcommand") };

        term::verbose::set(verbose != 0);
        // If `-vv` is passed, propagate `-v` to cargo.
        if verbose > 1 {
            cargo_args.push(format!("-{}", "v".repeat(verbose - 1)));
        }
        if let Some(color) = color {
            cargo_args.push("--color".to_owned());
            cargo_args.push(color.as_str().to_owned());
        }
        if let Some(path) = &manifest_path {
            cargo_args.push("--manifest-path".to_owned());
            cargo_args.push(path.clone());
        }

        Ok(Self { no_private, subcommand, manifest_path, cargo_args, rest })
    }
}

fn format_flag(flag: &lexopt::Arg<'_>) -> String {
    match flag {
        Long(flag) => format!("--{flag}"),
        Short(flag) => format!("-{flag}"),
        Value(_) => unreachable!(),
    }
}

#[cold]
#[inline(never)]
fn multi_arg(flag: &lexopt::Arg<'_>) -> Result<()> {
    let flag = &format_flag(flag);
    bail!("argument '{flag}' was provided more than once, but cannot be used multiple times")
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        io::Write,
        path::Path,
        process::{Command, Stdio},
    };

    use anyhow::Result;

    use super::USAGE;
    use crate::fs;

    #[track_caller]
    fn assert_diff(expected_path: impl AsRef<Path>, actual: impl AsRef<str>) {
        let actual = actual.as_ref();
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest_dir =
            manifest_dir.strip_prefix(env::current_dir().unwrap()).unwrap_or(manifest_dir);
        let expected_path = &manifest_dir.join(expected_path);
        if !expected_path.is_file() {
            fs::write(expected_path, "").unwrap();
        }
        let expected = fs::read_to_string(expected_path).unwrap();
        if expected != actual {
            if env::var_os("CI").is_some() {
                let mut child = Command::new("git")
                    .args(["--no-pager", "diff", "--no-index", "--"])
                    .arg(expected_path)
                    .arg("-")
                    .stdin(Stdio::piped())
                    .spawn()
                    .unwrap();
                child.stdin.as_mut().unwrap().write_all(actual.as_bytes()).unwrap();
                assert!(!child.wait().unwrap().success());
                // patch -p1 <<'EOF' ... EOF
                panic!("assertion failed; please run test locally and commit resulting changes, or apply above diff as patch");
            } else {
                fs::write(expected_path, actual).unwrap();
            }
        }
    }

    #[test]
    fn update_readme() -> Result<()> {
        let new = USAGE;
        let path = &Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
        let base = fs::read_to_string(path)?;
        let mut out = String::with_capacity(base.capacity());
        let mut lines = base.lines();
        let mut start = false;
        let mut end = false;
        while let Some(line) = lines.next() {
            out.push_str(line);
            out.push('\n');
            if line == "<!-- readme-long-help:start -->" {
                start = true;
                out.push_str("```console\n");
                out.push_str("$ cargo minimal-versions --help\n");
                out.push_str(new);
                for line in &mut lines {
                    if line == "<!-- readme-long-help:end -->" {
                        out.push_str("```\n");
                        out.push_str(line);
                        out.push('\n');
                        end = true;
                        break;
                    }
                }
            }
        }
        if start && end {
            assert_diff(path, out);
        } else if start {
            panic!("missing `<!-- readme-long-help:end -->` comment in README.md");
        } else {
            panic!("missing `<!-- readme-long-help:start -->` comment in README.md");
        }
        Ok(())
    }
}
