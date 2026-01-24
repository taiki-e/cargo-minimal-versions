// SPDX-License-Identifier: Apache-2.0 OR MIT

// Adapted from https://github.com/taiki-e/cargo-no-dev-deps

use std::{
    collections::{BTreeSet, HashSet},
    path::Path,
};

use anyhow::{Context as _, Result, bail, format_err};

use crate::{
    cli::{Args, DetachPathDeps},
    fs,
    metadata::Metadata,
    restore, term,
};

type ParseResult<T> = Result<T, &'static str>;

// Adapted from https://github.com/taiki-e/cargo-hack
// Cargo manifest
// https://doc.rust-lang.org/nightly/cargo/reference/manifest.html
pub(crate) struct Manifest {
    raw: String,
    doc: toml_edit::DocumentMut,
    pub(crate) package: Package,
}

impl Manifest {
    pub(crate) fn new(path: &Path, metadata_cargo_version: u32) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        let doc: toml_edit::DocumentMut = raw
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
    fn from_table(doc: &toml_edit::DocumentMut, metadata_cargo_version: u32) -> ParseResult<Self> {
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

pub(crate) fn with(
    metadata: &Metadata,
    args: &Args,
    no_dev_deps: bool,
    f: impl FnOnce() -> Result<()>,
) -> Result<()> {
    // TODO: provide option to keep updated Cargo.lock
    let restore_lockfile = true;
    let no_private = args.no_private;
    let restore = restore::Manager::new();
    let workspace_root = &metadata.workspace_root;
    let root_manifest = &workspace_root.join("Cargo.toml");
    let mut root_crate = None;
    let mut private_crates = BTreeSet::new();
    let modify_deps = |doc: &mut toml_edit::DocumentMut, manifest_path: &Path| {
        if term::verbose() {
            info!("modifying dependencies in {}", manifest_path.display());
        }
        remove_dev_deps(doc);
        if let Some(mode) = args.detach_path_deps {
            detach_path_deps(doc, mode);
        }
    };
    for &id in &metadata.workspace_members {
        let package = &metadata[id];
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
            private_crates.insert(manifest_path);
        } else if is_root && no_private {
            root_crate = Some(manifest);
            // This case is handled in the if block after loop.
        } else if no_dev_deps {
            let manifest = match manifest {
                Some(manifest) => manifest,
                None => Manifest::new(manifest_path, metadata.cargo_version)?,
            };
            let mut doc = manifest.doc;
            modify_deps(&mut doc, manifest_path);
            restore.register(manifest.raw, manifest_path);
            fs::write(manifest_path, doc.to_string())?;
        } else if args.detach_path_deps.is_some() {
            bail!(
                "--detach-path-deps is currently unsupported on subcommand that requires dev-dependencies: {}",
                args.subcommand.as_str()
            );
        }
    }
    let has_root_crate = root_crate.is_some();
    if no_private && (no_dev_deps && has_root_crate || !private_crates.is_empty()) {
        let manifest_path = root_manifest;
        let (mut doc, orig) = match root_crate {
            Some(Some(manifest)) => (manifest.doc, manifest.raw),
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
        if !private_crates.is_empty() {
            if term::verbose() {
                info!("removing private crates from {}", manifest_path.display());
            }
            remove_private_crates(&mut doc, workspace_root, private_crates);
        }
        restore.register(orig, manifest_path);
        fs::write(manifest_path, doc.to_string())?;
    }
    if restore_lockfile {
        let lockfile = &workspace_root.join("Cargo.lock");
        if lockfile.exists() {
            restore.register(fs::read(lockfile)?, lockfile);
        }
    }
    // TODO: emit error if there is no remaining crates due to --no-private (currently error is emitted by cargo update -Z ...)

    f()?;

    // Restore original Cargo.toml and Cargo.lock.
    restore.restore_all();

    Ok(())
}

fn remove_dev_deps(doc: &mut toml_edit::DocumentMut) {
    // Collect dependency names from [dependencies], [build-dependencies], [target.'...'.dependencies], and [target.'...'.build-dependencies].
    let mut keeping_features = HashSet::new();
    let mut collect_features = |table: &dyn toml_edit::TableLike| {
        for key in ["dependencies", "build-dependencies"] {
            if let Some(table) = table.get(key).and_then(toml_edit::Item::as_table_like) {
                keeping_features.reserve(table.len());
                for (name, _) in table.iter() {
                    keeping_features.insert(name.to_owned());
                }
            }
        }
    };
    let table = doc.as_table();
    collect_features(table);
    if let Some(table) = table.get("target").and_then(toml_edit::Item::as_table_like) {
        for (_, val) in table.iter() {
            if let Some(table) = val.as_table_like() {
                collect_features(table);
            }
        }
    }

    // Remove [dev-dependencies] and [target.'...'.dev-dependencies], and collect dependency names from it.
    let table = doc.as_table_mut();
    let mut removing_features = HashSet::new();
    let mut remove_dev_deps = |table: &mut dyn toml_edit::TableLike| {
        let removed = table.remove("dev-dependencies");
        if let Some(table) = removed.as_ref().and_then(toml_edit::Item::as_table_like) {
            for (name, _) in table.iter() {
                if !keeping_features.contains(name) {
                    removing_features.insert(name.to_owned());
                }
            }
        }
    };
    remove_dev_deps(table);
    if let Some(table) = table.get_mut("target").and_then(toml_edit::Item::as_table_like_mut) {
        for (_, val) in table.iter_mut() {
            if let Some(table) = val.as_table_like_mut() {
                remove_dev_deps(table);
            }
        }
    }
    drop(keeping_features);

    // Remove dev-dependency-only dependency names from [features].
    if let Some(table) = table.get_mut("features").and_then(toml_edit::Item::as_table_like_mut) {
        let mut indices = vec![];
        for (_, val) in table.iter_mut() {
            if let Some(array) = val.as_array_mut() {
                for (i, v) in array.iter().enumerate() {
                    if let Some(v) = v.as_str() {
                        if let Some((name, _)) = v.split_once('/') {
                            if removing_features.contains(name) {
                                indices.push(i);
                            }
                        }
                    }
                }
                for i in indices.drain(..).rev() {
                    array.remove(i);
                }
            }
        }
    }
}

fn remove_private_crates(
    doc: &mut toml_edit::DocumentMut,
    workspace_root: &Path,
    mut private_crates: BTreeSet<&Path>,
) {
    let table = doc.as_table_mut();
    if let Some(workspace) = table.get_mut("workspace").and_then(toml_edit::Item::as_table_like_mut)
    {
        if let Some(members) = workspace.get_mut("members").and_then(toml_edit::Item::as_array_mut)
        {
            let mut i = 0;
            while i < members.len() {
                if let Some(member) = members.get(i).and_then(toml_edit::Value::as_str) {
                    let manifest_path = workspace_root.join(member).join("Cargo.toml");
                    if let Some(p) = private_crates.iter().find_map(|p| {
                        same_file::is_same_file(p, &manifest_path)
                            .ok()
                            .and_then(|v| if v { Some(*p) } else { None })
                    }) {
                        members.remove(i);
                        private_crates.remove(p);
                        continue;
                    }
                }
                i += 1;
            }
        }
        if private_crates.is_empty() {
            return;
        }
        // Handles the case that the members field contains glob.
        // TODO: test that it also works when public and private crates are nested.
        if let Some(exclude) = workspace.get_mut("exclude").and_then(toml_edit::Item::as_array_mut)
        {
            for private_crate in private_crates {
                exclude.push(private_crate.parent().unwrap().to_str().unwrap());
            }
        } else {
            workspace.insert(
                "exclude",
                toml_edit::Item::Value(toml_edit::Value::Array(
                    private_crates
                        .iter()
                        .map(|p| {
                            toml_edit::Value::String(toml_edit::Formatted::new(
                                p.parent().unwrap().to_str().unwrap().to_owned(),
                            ))
                        })
                        .collect::<toml_edit::Array>(),
                )),
            );
        }
    }
}

fn detach_path_deps(doc: &mut toml_edit::DocumentMut, mode: DetachPathDeps) {
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
                    let mut doc: toml_edit::DocumentMut = $input.parse().unwrap();
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

        test!(
            dep_future,
            r#"
[features]
f1 = ["d1/f", "d2/f", "d4/f"]
f2 = ["d3/f", "d2/f", "d1/f"]
f3 = ["d2/f"]

[dependencies]
d1 = "1"
[target.'cfg(unix)'.dependencies]
d3 = "1"
[dev-dependencies]
d1 = "1"
d2 = "1"
[target.'cfg(unix)'.dev-dependencies]
d4 = "1"
"#,
            r#"
[features]
f1 = ["d1/f"]
f2 = ["d3/f", "d1/f"]
f3 = []

[dependencies]
d1 = "1"
[target.'cfg(unix)'.dependencies]
d3 = "1"
"#
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
