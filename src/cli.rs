use std::{env, ffi::OsString};

use anyhow::{bail, format_err, Result};

use crate::term;

static USAGE: &str = "cargo-minimal-versions\n
Wrapper for proper use of -Z minimal-versions.
\nUSAGE:
    cargo minimal-versions <SUBCOMMAND> [CARGO_OPTIONS]
\nSUBCOMMANDS:
    build
    check
    test
    ...
";

pub(crate) struct Args {
    pub(crate) subcommand: String,
    pub(crate) cargo_args: Vec<String>,
    pub(crate) rest: Vec<String>,
}

impl Args {
    pub(crate) fn parse() -> Result<Self> {
        // rustc/cargo args must be valid Unicode
        fn handle_args(
            args: impl IntoIterator<Item = impl Into<OsString>>,
        ) -> impl Iterator<Item = Result<String>> {
            // Adapted from https://github.com/rust-lang/rust/blob/3bc9dd0dd293ab82945e35888ed6d7ab802761ef/compiler/rustc_driver/src/lib.rs#L1365-L1375.
            args.into_iter().enumerate().map(|(i, arg)| {
                arg.into().into_string().map_err(|arg| {
                    format_err!("argument {} is not valid Unicode: {:?}", i + 1, arg)
                })
            })
        }

        let mut raw_args = handle_args(env::args_os());
        raw_args.next(); // cargo
        match raw_args.next().transpose()? {
            Some(a) if a == "minimal-versions" => {}
            Some(a) => {
                bail!("expected subcommand 'minimal-versions', found argument '{}'", a)
            }
            None => {
                bail!("expected subcommand 'minimal-versions'")
            }
        }
        let mut args = vec![];
        let mut verbose = false;
        let mut subcommand = None;
        for arg in &mut raw_args {
            let arg = arg?;
            if arg == "--" {
                break;
            }
            if subcommand.is_none() && !arg.starts_with('-') {
                subcommand = Some(arg);
                continue;
            }
            if arg == "--verbose"
                || arg.starts_with("-v") && arg.as_bytes()[1..].iter().all(|&v| v == b'v')
            {
                verbose = true;
            }
            args.push(arg);
        }
        let rest = raw_args.collect::<Result<Vec<_>>>()?;

        let subcommand = match subcommand {
            Some(subcommand) => subcommand,
            None => {
                if args.iter().any(|a| matches!(&**a, "-h" | "--help")) {
                    print!("{}", USAGE);
                    std::process::exit(0);
                }
                if args.iter().any(|a| matches!(&**a, "-V" | "--version")) {
                    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                bail!("expected subcommand");
            }
        };

        if verbose {
            term::set_verbose();
        }
        // TODO: get --color flag from cargo_args
        term::set_coloring(None)?;

        Ok(Self { subcommand, cargo_args: args, rest })
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path};

    use anyhow::Result;

    use super::USAGE;

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
            fs::write(path, out)?;
        } else if start {
            panic!("missing `<!-- readme-long-help:end -->` comment in README.md");
        } else {
            panic!("missing `<!-- readme-long-help:start -->` comment in README.md");
        }
        Ok(())
    }
}
