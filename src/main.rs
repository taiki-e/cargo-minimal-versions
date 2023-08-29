#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub)]
#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

#[macro_use]
mod term;

#[macro_use]
mod process;

mod cargo;
mod cli;
mod manifest;
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
    // TODO: provide option to keep updated Cargo.lock
    let restore_lockfile = true;
    manifest::with(&ws.metadata, remove_dev_deps, args.no_private, restore_lockfile, || {
        // Update Cargo.lock to minimal version dependencies.
        let mut cargo = ws.cargo_nightly();
        cargo.args(["update", "-Z", "minimal-versions"]);
        info!("running {cargo}");
        cargo.run()?;

        let mut cargo = ws.cargo();
        // TODO: Provide a way to do this without using cargo-hack.
        cargo.arg("hack");
        cargo.args(args.cargo_args);
        if !args.rest.is_empty() {
            cargo.arg("--");
            cargo.args(args.rest);
        }
        info!("running {cargo}");
        cargo.run()
    })
}
