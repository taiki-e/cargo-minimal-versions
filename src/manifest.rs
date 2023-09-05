// Adapted from https://github.com/taiki-e/cargo-no-dev-deps

use std::path::Path;

use anyhow::{bail, Context as _, Result};
use cargo_metadata::Metadata;
use fs_err as fs;

use crate::{restore, term};

pub(crate) fn with(
    metadata: &Metadata,
    no_dev_deps: bool,
    no_private: bool,
    restore_lockfile: bool,
    f: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let restore = restore::Manager::new();
    let mut restore_handles = Vec::with_capacity(metadata.workspace_members.len());
    let workspace_root = metadata.workspace_root.as_std_path();
    let root_manifest = &workspace_root.join("Cargo.toml");
    let mut has_root_crate = false;
    let mut private_crates = vec![];
    for id in &metadata.workspace_members {
        let package = &metadata[id];
        let manifest_path = package.manifest_path.as_std_path();
        let is_root = manifest_path == root_manifest;
        has_root_crate |= is_root;
        // Publishing is unrestricted if null, and forbidden if an empty array.
        let is_private = package.publish.as_ref().map_or(false, Vec::is_empty);
        if is_private && no_private {
            if is_root {
                bail!("--no-private is not supported yet with workspace with private root crate");
            }
            private_crates.push(manifest_path);
        } else if is_root && no_private {
            // This case is handled in the if block after loop.
        } else if no_dev_deps {
            let orig = fs::read_to_string(manifest_path)?;
            let mut doc = orig.parse().with_context(|| {
                format!("failed to parse manifest `{}` as toml", manifest_path.display())
            })?;
            if term::verbose() {
                info!("removing dev-dependencies from {}", manifest_path.display());
            }
            remove_dev_deps(&mut doc);
            restore_handles.push(restore.push(orig, manifest_path));
            fs::write(manifest_path, doc.to_string())?;
        }
    }
    if no_private && (no_dev_deps && has_root_crate || !private_crates.is_empty()) {
        let manifest_path = root_manifest;
        let orig = fs::read_to_string(manifest_path)?;
        let mut doc = orig.parse().with_context(|| {
            format!("failed to parse manifest `{}` as toml", manifest_path.display())
        })?;
        if no_dev_deps && has_root_crate {
            if term::verbose() {
                info!("removing dev-dependencies from {}", manifest_path.display());
            }
            remove_dev_deps(&mut doc);
        }
        if !private_crates.is_empty() {
            if term::verbose() {
                info!("removing private crates from {}", manifest_path.display());
            }
            remove_private_crates(&mut doc, workspace_root, &private_crates)?;
        }
        restore_handles.push(restore.push(orig, manifest_path));
        fs::write(manifest_path, doc.to_string())?;
    }
    if restore_lockfile {
        let lockfile = &workspace_root.join("Cargo.lock");
        if lockfile.exists() {
            restore_handles.push(restore.push(fs::read_to_string(lockfile)?, lockfile));
        }
    }

    f()?;

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

#[cfg(test)]
mod tests {
    use super::remove_dev_deps;

    macro_rules! test {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let mut doc: toml_edit::Document = $input.parse().unwrap();
                remove_dev_deps(&mut doc);
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
