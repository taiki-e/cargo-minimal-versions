#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub)]
#![warn(
    clippy::default_trait_access,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::wildcard_imports
)]

#[macro_use]
mod term;

#[macro_use]
mod process;

mod cli;
mod context;
mod remove_dev_deps;
mod restore;

use anyhow::Result;
use fs_err as fs;

use crate::{cli::Args, context::Context};

fn main() {
    if let Err(e) = try_main() {
        error!("{:#}", e);
    }
    if term::has_error() {
        std::process::exit(1)
    }
}

fn try_main() -> Result<()> {
    let args = Args::parse()?;
    // TODO: pass manifest path if --manifest-path option passed
    let cx = &Context::new(None)?;

    // Remove dev-dependencies from Cargo.toml to prevent the next `cargo update`
    // from determining minimal versions based on dev-dependencies.
    let remove_dev_deps = !matches!(&*args.subcommand, "test" | "t")
        && !args.cargo_args.iter().any(|a| {
            matches!(
                &**a,
                "--all-targets"
                    | "--bench"
                    | "--benches"
                    | "--example"
                    | "--examples"
                    | "--test"
                    | "--tests"
            ) || a.starts_with("--bench=")
                || a.starts_with("--example=")
                || a.starts_with("--test=")
        });
    // TODO: provide option to keep updated Cargo.lock
    let restore_lockfile = true;
    let restore = restore::Manager::new();
    let mut restore_handles = Vec::with_capacity(cx.metadata.workspace_members.len());
    if remove_dev_deps {
        for id in &cx.metadata.workspace_members {
            let manifest_path = &cx.metadata[id].manifest_path;
            let orig = fs::read_to_string(manifest_path)?;
            let new = remove_dev_deps::remove_dev_deps(&orig);
            restore_handles.push(restore.push(orig, manifest_path.as_std_path()));
            if term::verbose() {
                info!("removing dev-dependencies from {}", manifest_path);
            }
            fs::write(manifest_path, new)?;
        }
    }
    if restore_lockfile {
        let lockfile = &cx.metadata.workspace_root.join("Cargo.lock");
        if lockfile.exists() {
            restore_handles
                .push(restore.push(fs::read_to_string(lockfile)?, lockfile.as_std_path()));
        }
    }

    // Update Cargo.lock to minimal version dependencies.
    let mut cargo = cx.cargo_nightly();
    cargo.args(&["update", "-Z", "minimal-versions"]);
    info!("running {}", cargo);
    cargo.run()?;

    let mut cargo = cx.cargo();
    // TODO: Provide a way to do this without using cargo-hack.
    cargo.arg("hack");
    cargo.arg(args.subcommand);
    cargo.args(args.cargo_args);
    if !args.rest.is_empty() {
        cargo.arg("--");
        cargo.args(args.rest);
    }
    info!("running {}", cargo);
    cargo.run()?;

    // Restore original Cargo.toml and Cargo.lock.
    drop(restore_handles);

    Ok(())
}
