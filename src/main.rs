// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

#[macro_use]
mod term;

#[macro_use]
mod process;

mod cargo;
mod cli;
mod fs;
mod manifest;
mod metadata;
mod restore;

use std::{env, path::Path, process::ExitCode};

use anyhow::{Result, bail};

use crate::{cargo::Workspace, cli::Args};

fn main() -> ExitCode {
    term::init_coloring();
    if let Err(e) = try_main() {
        error!("{e:#}");
    }
    if term::error()
        || term::warn() && env::var_os("CARGO_MINIMAL_VERSIONS_DENY_WARNINGS").is_some()
    {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn try_main() -> Result<()> {
    let Some(args) = Args::parse()? else { return Ok(()) };
    let ws = Workspace::new(args.manifest_path.as_deref(), args.direct)?;

    // Remove dev-dependencies from Cargo.toml to prevent the next `cargo update`
    // from determining minimal versions based on dev-dependencies.
    let remove_dev_deps = !args.subcommand.always_needs_dev_deps()
        && !args.cargo_args.iter().any(|a| match &**a {
            "--example" | "--examples" | "--test" | "--tests" | "--bench" | "--benches"
            | "--all-targets" => true,
            _ => {
                a.starts_with("--example=") || a.starts_with("--test=") || a.starts_with("--bench=")
            }
        });
    if !remove_dev_deps && args.detach_path_deps.is_some() {
        bail!(
            "--detach-path-deps is currently unsupported on subcommand that requires dev-dependencies: {}",
            args.subcommand.as_str()
        );
    }
    manifest::with(&ws.metadata, &args, remove_dev_deps, |ids, detach_workspace| {
        let update_lockfile = |dir: Option<&Path>| {
            // Update Cargo.lock to minimal version dependencies.
            let mut cargo = ws.cargo_nightly();
            cargo.arg("update");
            if args.quiet {
                cargo.arg("-q");
            }
            let mode = if args.direct { "direct-minimal-versions" } else { "minimal-versions" };
            cargo.args(["-Z", mode]);
            if let Some(dir) = dir {
                cargo.dir(dir);
                info!("running {cargo} on {}", dir.display());
            } else {
                info!("running {cargo}");
            }
            cargo.run()
        };

        let mut cargo = ws.cargo();
        // TODO: Provide a way to do this without using cargo-hack. https://github.com/taiki-e/cargo-minimal-versions/issues/5
        cargo.arg("hack");
        cargo.args(&args.cargo_args);
        if !detach_workspace && args.workspace {
            cargo.arg("--workspace");
        }
        if let Some(dir) = &args.target_dir {
            cargo.arg("--target-dir");
            if detach_workspace {
                cargo.arg(fs::canonicalize(dir)?);
            } else {
                cargo.arg(dir);
            }
        } else if detach_workspace {
            cargo.arg("--target-dir");
            cargo.arg(&ws.metadata.target_directory);
        }
        if !args.rest.is_empty() {
            cargo.arg("--");
            cargo.args(&args.rest);
        }
        if detach_workspace {
            // TODO: respect --package/--exclude/--manifest-dir options
            for id in ids {
                let manifest_dir = ws.metadata[id].manifest_path.parent().unwrap();
                update_lockfile(Some(manifest_dir))?;
                cargo.dir(manifest_dir);
                info!("running {cargo} on {}", manifest_dir.display());
                cargo.run()?;
            }
            Ok(())
        } else {
            update_lockfile(None)?;
            info!("running {cargo}");
            cargo.run()
        }
    })
}
