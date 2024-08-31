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

use std::env;

use anyhow::Result;

use crate::{cargo::Workspace, cli::Args};

fn main() {
    term::init_coloring();
    if let Err(e) = try_main() {
        error!("{e:#}");
    }
    if term::error()
        || term::warn() && env::var_os("CARGO_MINIMAL_VERSIONS_DENY_WARNINGS").is_some()
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
    manifest::with(&ws.metadata, &args, remove_dev_deps, || {
        // Update Cargo.lock to minimal version dependencies.
        let mut cargo = ws.cargo_nightly();
        if args.direct {
            cargo.args(["update", "-Z", "direct-minimal-versions"]);
        } else {
            cargo.args(["update", "-Z", "minimal-versions"]);
        }
        info!("running {cargo}");
        cargo.run()?;

        let mut cargo = ws.cargo();
        // TODO: Provide a way to do this without using cargo-hack. https://github.com/taiki-e/cargo-minimal-versions/issues/5
        cargo.arg("hack");
        cargo.args(&args.cargo_args);
        if !args.rest.is_empty() {
            cargo.arg("--");
            cargo.args(&args.rest);
        }
        info!("running {cargo}");
        cargo.run()
    })
}
