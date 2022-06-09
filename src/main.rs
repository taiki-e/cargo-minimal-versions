#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub)]
#![warn(clippy::default_trait_access, clippy::wildcard_imports)]

#[macro_use]
mod term;

#[macro_use]
mod process;

mod cargo;
mod cli;
mod restore;

use std::env;

use anyhow::{Context as _, Result};
use fs_err as fs;

use crate::{cargo::Workspace, cli::Args};

fn main() {
    if let Err(e) = try_main() {
        error!("{:#}", e);
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
    let restore = restore::Manager::new();
    let mut restore_handles = Vec::with_capacity(ws.metadata.workspace_members.len());
    let detach_workspace = ws.metadata.workspace_members.len() > 1;
    if remove_dev_deps || restore_lockfile || detach_workspace {
        info!(
            "cargo-minimal-versions modifies {} while running and restores it when finished; \
             note that any changes you made to {} during running will not be preserved",
            if remove_dev_deps || detach_workspace {
                "`Cargo.toml` and `Cargo.lock`"
            } else {
                "`Cargo.lock`"
            },
            if restore_lockfile || detach_workspace { "those files" } else { "`Cargo.toml`" }
        );
    }
    if remove_dev_deps {
        let mut root_manifest = if detach_workspace {
            Some(ws.metadata.workspace_root.join("Cargo.toml"))
        } else {
            None
        };
        for id in &ws.metadata.workspace_members {
            let manifest_path = &ws.metadata[id].manifest_path;
            let orig = fs::read_to_string(manifest_path)?;
            let mut doc = orig
                .parse()
                .with_context(|| format!("failed to parse manifest `{}` as toml", manifest_path))?;
            self::remove_dev_deps(&mut doc);
            if args.detach_path_deps {
                detach_path_deps(&mut doc);
            }
            if root_manifest.as_ref() == Some(manifest_path) {
                root_manifest = None;
                detach_workspace_members(&mut doc);
            } else if detach_workspace {
                to_workspace(&mut doc);
                // TODO: remove Cargo.lock
            }
            restore_handles.push(restore.push(orig, manifest_path.as_std_path()));
            if term::verbose() {
                info!("modifying {}", manifest_path);
            }
            fs::write(manifest_path, doc.to_string())?;
        }
        if let Some(manifest_path) = &root_manifest {
            let orig = fs::read_to_string(manifest_path)?;
            let mut doc = orig.parse()?;
            detach_workspace_members(&mut doc);
            restore_handles.push(restore.push(orig, manifest_path.as_std_path()));
            if term::verbose() {
                info!("modifying {}", manifest_path);
            }
            fs::write(manifest_path, doc.to_string())?;
        }
    } else if args.detach_path_deps {
        warn!("--detach-path-deps is ignored on {}", args.subcommand.as_str());
    }
    if restore_lockfile {
        let lockfile = &ws.metadata.workspace_root.join("Cargo.lock");
        if lockfile.exists() {
            restore_handles
                .push(restore.push(fs::read_to_string(lockfile)?, lockfile.as_std_path()));
        } else {
            // remove_handles.push()
        }
    }

    // Update Cargo.lock to minimal version dependencies.
    let mut cargo = ws.cargo_nightly();
    cargo.args(&["update", "-Z", "minimal-versions"]);
    if detach_workspace {
        // TODO: respect --package/--exclude options
        // TODO: ignore private crate when --ignore-private flag passed
        for id in &ws.metadata.workspace_members {
            let manifest_dir = ws.metadata[id].manifest_path.parent().unwrap();
            cargo.dir(manifest_dir);
            info!("running {} on {}", cargo, manifest_dir);
            cargo.run()?;
        }
    } else {
        info!("running {}", cargo);
        cargo.run()?;
    }

    let mut cargo = ws.cargo();
    // TODO: Provide a way to do this without using cargo-hack.
    cargo.arg("hack");
    cargo.args(args.cargo_args);
    if !detach_workspace && args.workspace {
        cargo.arg("--workspace");
    }
    if !args.rest.is_empty() {
        cargo.arg("--");
        cargo.args(args.rest);
    }
    if detach_workspace {
        // TODO: respect --package/--exclude options
        // TODO: ignore private crate when --ignore-private flag passed
        for id in &ws.metadata.workspace_members {
            let manifest_dir = ws.metadata[id].manifest_path.parent().unwrap();
            cargo.dir(manifest_dir);
            info!("running {} on {}", cargo, manifest_dir);
            cargo.run()?;
        }
    } else {
        info!("running {}", cargo);
        cargo.run()?;
    }

    // Restore original Cargo.toml and Cargo.lock.
    drop(restore_handles);

    Ok(())
}

fn remove_dev_deps(doc: &mut toml_edit::Document) {
    const KEY: &str = "dev-dependencies";
    let table = doc.as_table_mut();
    table.remove(KEY);
    if let Some(table) = table.get_mut("target").and_then(toml_edit::Item::as_table_like_mut) {
        for (_, val) in table.iter_mut() {
            if let Some(table) = val.as_table_like_mut() {
                table.remove(KEY);
            }
        }
    }
}

fn detach_workspace_members(doc: &mut toml_edit::Document) {
    let table = doc.as_table_mut();
    if let Some(table) = table.get_mut("workspace").and_then(toml_edit::Item::as_table_like_mut) {
        table.remove("members");
    }
}

fn to_workspace(doc: &mut toml_edit::Document) {
    let table = doc.as_table_mut();
    table.entry("workspace").or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::default()));
}

fn detach_path_deps(doc: &mut toml_edit::Document) {
    // Omitting version in dev-dependencies is fine:
    // https://github.com/rust-lang/cargo/pull/7333
    // https://github.com/rust-lang/futures-rs/pull/2305
    const KIND: &[&str] = &["build-dependencies", "dependencies"];
    fn remove_path(deps: &mut toml_edit::Item) {
        if let Some(deps) = deps.as_table_like_mut() {
            for (_name, dep) in deps.iter_mut() {
                if let Some(dep) = dep.as_table_like_mut() {
                    // without this check, we got "dependency specified without
                    // providing a local path, Git repository, or version to use" warning
                    if dep.get("version").is_some() || dep.get("git").is_some() {
                        dep.remove("path");
                    }
                }
            }
        }
    }
    for key in KIND {
        if let Some(deps) = doc.get_mut(key) {
            remove_path(deps);
        }
    }
    if let Some(table) = doc.get_mut("target").and_then(toml_edit::Item::as_table_like_mut) {
        for (_key, val) in table.iter_mut() {
            if let Some(table) = val.as_table_like_mut() {
                for key in KIND {
                    if let Some(deps) = table.get_mut(key) {
                        remove_path(deps);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    mod remove_dev_deps {
        macro_rules! test {
            ($name:ident, $input:expr, $expected:expr) => {
                #[test]
                fn $name() {
                    let mut doc = $input.parse().unwrap();
                    super::super::remove_dev_deps(&mut doc);
                    assert_eq!($expected, doc.to_string());
                }
            };
        }

        test!(
            a,
            "\
[package]
[dependencies]
[[example]]
[dev-dependencies.opencl]
[dev-dependencies]",
            "\
[package]
[dependencies]
[[example]]
"
        );

        test!(
            b,
            "\
[package]
[dependencies]
[[example]]
[dev-dependencies.opencl]
[dev-dependencies]
",
            "\
[package]
[dependencies]
[[example]]
"
        );

        test!(
            c,
            "\
[dev-dependencies]
foo = { features = [] }
bar = \"0.1\"
",
            "\
         "
        );

        test!(
            d,
            "\
[dev-dependencies.foo]
features = []

[dev-dependencies]
bar = { features = [], a = [] }

[dependencies]
bar = { features = [], a = [] }
",
            "
[dependencies]
bar = { features = [], a = [] }
"
        );

        test!(
            many_lines,
            "\
[package]\n\n

[dev-dependencies.opencl]


[dev-dependencies]
",
            "\
[package]
"
        );

        test!(
            target_deps1,
            "\
[package]

[target.'cfg(unix)'.dev-dependencies]

[dependencies]
",
            "\
[package]

[dependencies]
"
        );

        test!(
            target_deps2,
            "\
[package]

[target.'cfg(unix)'.dev-dependencies]
foo = \"0.1\"

[target.'cfg(unix)'.dev-dependencies.bar]

[dev-dependencies]
foo = \"0.1\"

[target.'cfg(unix)'.dependencies]
foo = \"0.1\"
",
            "\
[package]

[target.'cfg(unix)'.dependencies]
foo = \"0.1\"
"
        );

        test!(
            target_deps3,
            "\
[package]

[target.'cfg(unix)'.dependencies]

[dev-dependencies]
",
            "\
[package]

[target.'cfg(unix)'.dependencies]
"
        );

        test!(
            target_deps4,
            "\
[package]

[target.'cfg(unix)'.dev-dependencies]
",
            "\
[package]
"
        );

        // NOTE: `foo = [[dev-dependencies]]` is not valid TOML format.
        test!(
            not_table_multi_line,
            "\
[package]
foo = [
    ['dev-dependencies'],
    [\"dev-dependencies\"]
]
",
            "\
[package]
foo = [
    ['dev-dependencies'],
    [\"dev-dependencies\"]
]
"
        );
    }

    mod detach_path_deps {
        macro_rules! test {
            ($name:ident, $input:expr, $expected:expr) => {
                #[test]
                fn $name() {
                    let mut doc = $input.parse().unwrap();
                    super::super::detach_path_deps(&mut doc);
                    assert_eq!($expected, doc.to_string());
                }
            };
        }

        test!(
            a,
            "\
[dependencies]
a = { version = '1', path = 'p' }
g = { path = 'p' }
[build-dependencies]
b = { path = 'p', version = '1' }
[dev-dependencies]
c = { path = 'p', version = '1' }
[dependencies.d]
path = 'p'
version = '1'
[build-dependencies.e]
version = '1'
path = 'p'
[dev-dependencies.f]
version = '1'
path = 'p'
",
            "\
[dependencies]
a = { version = '1'}
g = { path = 'p' }
[build-dependencies]
b = { version = '1' }
[dev-dependencies]
c = { path = 'p', version = '1' }
[dependencies.d]
version = '1'
[build-dependencies.e]
version = '1'
[dev-dependencies.f]
version = '1'
path = 'p'
"
        );

        test!(
            b,
            "\
[target.a.dependencies]
a = { version = '1', path = 'p' }
g = { path = 'p' }
[target.b.build-dependencies]
b = { path = 'p', version = '1' }
[target.c.dev-dependencies]
c = { path = 'p', version = '1' }
[target.c.dependencies.d]
path = 'p'
version = '1'
[target.b.build-dependencies.e]
version = '1'
path = 'p'
[target.a.dev-dependencies.f]
version = '1'
path = 'p'
",
            "\
[target.a.dependencies]
a = { version = '1'}
g = { path = 'p' }
[target.b.build-dependencies]
b = { version = '1' }
[target.c.dev-dependencies]
c = { path = 'p', version = '1' }
[target.c.dependencies.d]
version = '1'
[target.b.build-dependencies.e]
version = '1'
[target.a.dev-dependencies.f]
version = '1'
path = 'p'
"
        );
    }
}
