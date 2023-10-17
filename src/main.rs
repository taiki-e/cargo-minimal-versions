// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub, clippy::pedantic)]
#![allow(clippy::too_many_lines, clippy::single_match_else, clippy::type_complexity)]

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

use std::{env, path::Path};

use anyhow::{bail, Result};

use crate::{cargo::Workspace, cli::Args};

fn main() {
    term::init_coloring();
    if let Err(e) = try_main() {
        error!("{e:#}");
    }
    if term::error()
        || term::warn()
            && env::var_os("CARGO_MINIMAL_VERSIONS_DENY_WARNINGS").filter(|v| v == "true").is_some()
    {
        std::process::exit(1)
    }
}

fn try_main() -> Result<()> {
    let args = Args::parse()?;
    let ws = Workspace::new(args.manifest_path.as_deref())?;

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
            cargo.args(["update", "-Z", "minimal-versions"]);
            if let Some(dir) = dir {
                cargo.dir(dir);
                info!("running {cargo} on {}", dir.display());
            } else {
                info!("running {cargo}");
            }
            cargo.run()
        };

        let mut cargo = ws.cargo();
        // TODO: Provide a way to do this without using cargo-hack.
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
                let manifest_dir = ws.metadata.packages[id].manifest_path.parent().unwrap();
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
