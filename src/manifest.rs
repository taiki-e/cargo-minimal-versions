// SPDX-License-Identifier: Apache-2.0 OR MIT

// Adapted from https://github.com/taiki-e/cargo-no-dev-deps

use std::path::Path;

use anyhow::{bail, format_err, Context as _, Result};

use crate::{
    cli::{Args, DetachPathDeps},
    fs,
    metadata::{Metadata, PackageId},
    restore, term,
};

type ParseResult<T> = Result<T, &'static str>;

// Adapted from https://github.com/taiki-e/cargo-hack
// Cargo manifest
// https://doc.rust-lang.org/nightly/cargo/reference/manifest.html
pub(crate) struct Manifest {
    pub(crate) raw: String,
    pub(crate) doc: toml_edit::Document,
    pub(crate) package: Package,
}

impl Manifest {
    pub(crate) fn new(path: &Path, metadata_cargo_version: u32) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        let doc: toml_edit::Document = raw
            .parse()
            .with_context(|| format!("failed to parse manifest `{}` as toml", path.display()))?;
        let package = Package::from_table(&doc, metadata_cargo_version).map_err(|s| {
            format_err!("failed to parse `{s}` field from manifest `{}`", path.display())
        })?;
        Ok(Self { raw, doc, package })
    }
}

pub(crate) struct Package {
    // `metadata.package.publish` requires Rust 1.39
    pub(crate) publish: Option<bool>,
}

impl Package {
    fn from_table(doc: &toml_edit::Document, metadata_cargo_version: u32) -> ParseResult<Self> {
        let package = doc.get("package").and_then(toml_edit::Item::as_table).ok_or("package")?;

        Ok(Self {
            // Publishing is unrestricted if `true` or the field is not
            // specified, and forbidden if `false` or the array is empty.
            publish: if metadata_cargo_version >= 39 {
                None // Use `metadata.package.publish` instead.
            } else {
                Some(match package.get("publish") {
                    None => true,
                    Some(toml_edit::Item::Value(toml_edit::Value::Boolean(b))) => *b.value(),
                    Some(toml_edit::Item::Value(toml_edit::Value::Array(a))) => !a.is_empty(),
                    Some(_) => return Err("publish"),
                })
            },
        })
    }
}

fn collect_manifests(
    metadata: &Metadata,
    no_private: bool,
) -> Result<(Vec<(&PackageId, Manifest)>, Vec<&Path>)> {
    let mut manifests = Vec::with_capacity(metadata.workspace_members.len());
    let mut private_crates = vec![];
    let workspace_root = &metadata.workspace_root;
    let root_manifest = &workspace_root.join("Cargo.toml");
    for id in &metadata.workspace_members {
        let package = &metadata.packages[id];
        let manifest_path = &*package.manifest_path;
        let is_root = manifest_path == root_manifest;
        let mut manifest = None;
        let is_private = if metadata.cargo_version >= 39 {
            !package.publish
        } else {
            let m = Manifest::new(manifest_path, metadata.cargo_version)?;
            let is_private = !m.package.publish.unwrap();
            manifest = Some(m);
            is_private
        };
        if is_private && no_private {
            if is_root {
                bail!("--no-private is not supported yet with workspace with private root crate");
            }
            private_crates.push(manifest_path);
            continue;
        }
        let manifest = match manifest {
            Some(manifest) => manifest,
            None => Manifest::new(manifest_path, metadata.cargo_version)?,
        };
        manifests.push((id, manifest));
    }
    Ok((manifests, private_crates))
}

pub(crate) fn with(
    metadata: &Metadata,
    args: &Args,
    no_dev_deps: bool,
    f: impl FnOnce(Vec<&PackageId>, bool) -> Result<()>,
) -> Result<()> {
    // TODO: provide option to keep updated Cargo.lock
    let restore_lockfile = true;
    let no_private = args.no_private;
    let restore = restore::Manager::new();
    let workspace_root = &metadata.workspace_root;
    let root_manifest = &workspace_root.join("Cargo.toml");
    let mut root_crate = None;
    let (manifests, private_crates) = collect_manifests(metadata, no_private)?;
    let detach_workspace = manifests.len() > 1;
    let modify_deps = |doc: &mut toml_edit::Document, manifest_path: &Path| {
        if term::verbose() {
            info!("modifying from {}", manifest_path.display());
        }
        remove_dev_deps(doc);
        if let Some(mode) = args.detach_path_deps {
            detach_path_deps(doc, mode);
        }
    };
    let mut ids = Vec::with_capacity(manifests.len());
    for (id, manifest) in manifests {
        ids.push(id);
        let package = &metadata.packages[id];
        let manifest_path = &*package.manifest_path;
        let is_root = manifest_path == root_manifest;
        if is_root && no_private {
            root_crate = Some(manifest);
            // This case is handled in the if block after loop.
        } else if no_dev_deps {
            let mut doc = manifest.doc;
            modify_deps(&mut doc, manifest_path);
            if detach_workspace {
                if is_root {
                    detach_workspace_members(&mut doc, workspace_root)?;
                } else {
                    to_workspace(&mut doc);
                    let lockfile = &manifest_path.parent().unwrap().join("Cargo.lock");
                    if lockfile.exists() {
                        restore.register(fs::read(lockfile)?, lockfile);
                    } else {
                        restore.register_remove(lockfile);
                    }
                }
            }
            restore.register(manifest.raw.into_bytes(), manifest_path);
            fs::write(manifest_path, doc.to_string())?;
        } else if args.detach_path_deps.is_some() {
            bail!(
                "--detach-path-deps is currently unsupported on subcommand that requires dev-dependencies: {}",
                args.subcommand.as_str()
            );
        } else if detach_workspace {
            let mut doc = manifest.doc;
            if is_root {
                detach_workspace_members(&mut doc, workspace_root)?;
            } else {
                to_workspace(&mut doc);
                let lockfile = &manifest_path.parent().unwrap().join("Cargo.lock");
                if lockfile.exists() {
                    restore.register(fs::read(lockfile)?, lockfile);
                } else {
                    restore.register_remove(lockfile);
                }
            }
            restore.register(manifest.raw.into_bytes(), manifest_path);
            fs::write(manifest_path, doc.to_string())?;
        }
    }
    let has_root_crate = root_crate.is_some();
    if no_private && (no_dev_deps && has_root_crate || !private_crates.is_empty()) {
        let manifest_path = root_manifest;
        let (mut doc, orig) = match root_crate {
            Some(manifest) => (manifest.doc, manifest.raw),
            _ => {
                let orig = fs::read_to_string(manifest_path)?;
                (
                    orig.parse().with_context(|| {
                        format!("failed to parse manifest `{}` as toml", manifest_path.display())
                    })?,
                    orig,
                )
            }
        };
        if no_dev_deps && has_root_crate {
            modify_deps(&mut doc, manifest_path);
        }
        if detach_workspace {
            detach_workspace_members(&mut doc, workspace_root)?;
        } else if !private_crates.is_empty() {
            if term::verbose() {
                info!("removing private crates from {}", manifest_path.display());
            }
            remove_private_crates(&mut doc, workspace_root, &private_crates)?;
        }
        restore.register(orig.into_bytes(), manifest_path);
        fs::write(manifest_path, doc.to_string())?;
    }
    if restore_lockfile {
        let lockfile = &workspace_root.join("Cargo.lock");
        if lockfile.exists() {
            restore.register(fs::read(lockfile)?, lockfile);
        }
    }

    f(ids, detach_workspace)?;

    // Restore original Cargo.toml and Cargo.lock.
    restore.restore_all();

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

fn remove_private_crates(
    doc: &mut toml_edit::Document,
    workspace_root: &Path,
    private_crates: &[&Path],
) -> Result<()> {
    let table = doc.as_table_mut();
    if let Some(workspace) = table.get_mut("workspace").and_then(toml_edit::Item::as_table_like_mut)
    {
        if let Some(members) = workspace.get_mut("members").and_then(toml_edit::Item::as_array_mut)
        {
            let mut i = 0;
            while i < members.len() {
                if let Some(member) = members.get(i).and_then(toml_edit::Value::as_str) {
                    let manifest_path = workspace_root.join(member).join("Cargo.toml");
                    if private_crates
                        .iter()
                        .find_map(|p| {
                            same_file::is_same_file(p, &manifest_path)
                                .map(|v| if v { Some(()) } else { None })
                                .transpose()
                        })
                        .transpose()?
                        .is_some()
                    {
                        members.remove(i);
                        continue;
                    }
                }
                i += 1;
            }
        }
    }
    Ok(())
}

fn detach_workspace_members(doc: &mut toml_edit::Document, workspace_root: &Path) -> Result<()> {
    let table = doc.as_table_mut();
    if let Some(table) = table.get_mut("workspace").and_then(toml_edit::Item::as_table_like_mut) {
        if let Some(members) = table.remove("members") {
            if let Some(exclude) = table.get_mut("exclude").and_then(toml_edit::Item::as_array_mut)
            {
                if let Some(members) = members.as_array() {
                    let root_manifest = &workspace_root.join("Cargo.toml");
                    for member in members {
                        if let Some(member) = member.as_str() {
                            let p = workspace_root.join(member).join("Cargo.toml");
                            if !same_file::is_same_file(p, root_manifest)? {
                                exclude.push(member);
                            }
                        }
                    }
                }
            } else {
                table.insert("exclude", members);
            }
        }
    }
    Ok(())
}

fn to_workspace(doc: &mut toml_edit::Document) {
    let table = doc.as_table_mut();
    table.entry("workspace").or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::default()));
}

fn detach_path_deps(doc: &mut toml_edit::Document, mode: DetachPathDeps) {
    // --detach-path-deps is currently only supported for subcommands that call remove_dev_deps.
    const KIND: &[&str] = &["build-dependencies", "dependencies"];
    fn remove_path(deps: &mut toml_edit::Item, mode: DetachPathDeps) {
        if let Some(deps) = deps.as_table_like_mut() {
            for (_name, dep) in deps.iter_mut() {
                if let Some(dep) = dep.as_table_like_mut() {
                    if let Some(req) = dep.get("version") {
                        if mode == DetachPathDeps::SkipExact {
                            if let Some(req) =
                                req.as_str().and_then(|s| semver::VersionReq::parse(s).ok())
                            {
                                if req.comparators.len() == 1 {
                                    let req = req.comparators.first().unwrap();
                                    if req.op == semver::Op::Exact
                                        && req.patch.is_some()
                                        // TODO
                                        && req.pre.is_empty()
                                    {
                                        continue;
                                    }
                                }
                            }
                        }
                        dep.remove("path");
                    } else if dep.get("git").is_some() {
                        dep.remove("path");
                    } else {
                        // Do not remove path deps in this case. When removed,
                        // we will got "dependency specified without providing
                        // a local path, Git repository, or version to use" warning.
                    }
                }
            }
        }
    }
    for key in KIND {
        if let Some(deps) = doc.get_mut(key) {
            remove_path(deps, mode);
        }
    }
    if let Some(table) = doc.get_mut("target").and_then(toml_edit::Item::as_table_like_mut) {
        for (_key, val) in table.iter_mut() {
            if let Some(table) = val.as_table_like_mut() {
                for key in KIND {
                    if let Some(deps) = table.get_mut(key) {
                        remove_path(deps, mode);
                    }
                }
            }
        }
    }
    // [workspace.dependencies]
    if let Some(table) = doc.get_mut("workspace").and_then(toml_edit::Item::as_table_like_mut) {
        if let Some(deps) = table.get_mut("dependencies") {
            remove_path(deps, mode);
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
                    let mut doc: toml_edit::Document = $input.parse().unwrap();
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
[dev-dependencies.serde]
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
[dev-dependencies.serde]
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

[dev-dependencies.serde]


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
            ($name:ident, $mode:ident, $input:expr, $expected:expr) => {
                #[test]
                fn $name() {
                    let mut doc = $input.parse().unwrap();
                    super::super::detach_path_deps(&mut doc, crate::cli::DetachPathDeps::$mode);
                    assert_eq!($expected, doc.to_string());
                }
            };
        }

        test!(
            deps,
            All,
            "\
[dependencies]
a = { version = '1', path = 'p' }
g = { path = 'p' }
h = { version = '=1', path = 'p' }
i = { version = '=1.2', path = 'p' }
j = { version = '=1.2.3', path = 'p' }
k = { version = '=1.2.3-alpha', path = 'p' }
l = { version = '=1.2.3-alpha.1', path = 'p' }
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
h = { version = '=1'}
i = { version = '=1.2'}
j = { version = '=1.2.3'}
k = { version = '=1.2.3-alpha'}
l = { version = '=1.2.3-alpha.1'}
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
            target_deps,
            SkipExact,
            "\
[target.a.dependencies]
a = { version = '1', path = 'p' }
g = { path = 'p' }
h = { version = '=1', path = 'p' }
i = { version = '=1.2', path = 'p' }
j = { version = '=1.2.3', path = 'p' }
k = { version = '=1.2.3-alpha', path = 'p' }
l = { version = '=1.2.3-alpha.1', path = 'p' }
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
h = { version = '=1'}
i = { version = '=1.2'}
j = { version = '=1.2.3', path = 'p' }
k = { version = '=1.2.3-alpha'}
l = { version = '=1.2.3-alpha.1'}
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

        test!(
            workspace_deps,
            All,
            "\
[workspace.dependencies]
a = { version = '1', path = 'p' }
g = { path = 'p' }
h = { version = '=1', path = 'p' }
i = { version = '=1.2', path = 'p' }
j = { version = '=1.2.3', path = 'p' }
k = { version = '=1.2.3-alpha', path = 'p' }
l = { version = '=1.2.3-alpha.1', path = 'p' }
[workspace.dependencies.d]
path = 'p'
version = '1'
",
            "\
[workspace.dependencies]
a = { version = '1'}
g = { path = 'p' }
h = { version = '=1'}
i = { version = '=1.2'}
j = { version = '=1.2.3'}
k = { version = '=1.2.3-alpha'}
l = { version = '=1.2.3-alpha.1'}
[workspace.dependencies.d]
version = '1'
"
        );
    }
}
