use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{format_err, Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};

use crate::process::ProcessBuilder;

pub(crate) struct Workspace {
    pub(crate) metadata: cargo_metadata::Metadata,
    cargo: PathBuf,
    nightly: bool,
}

impl Workspace {
    pub(crate) fn new(manifest_path: Option<&Utf8Path>) -> Result<Self> {
        let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let rustc = rustc_path(&cargo);
        let nightly = rustc_version(&rustc)?;

        // Metadata
        let current_manifest_path = package_root(&cargo, manifest_path)?;
        let metadata = metadata(&cargo, &current_manifest_path)?;

        Ok(Self { cargo: cargo.into(), nightly, metadata })
    }

    pub(crate) fn cargo(&self) -> ProcessBuilder {
        cmd!(&self.cargo)
    }

    pub(crate) fn cargo_nightly(&self) -> ProcessBuilder {
        if self.nightly {
            self.cargo()
        } else {
            cmd!("cargo", "+nightly")
        }
    }
}

fn rustc_path(cargo: impl AsRef<Path>) -> PathBuf {
    // When toolchain override shorthand (`+toolchain`) is used, `rustc` in
    // PATH and `CARGO` environment variable may be different toolchains.
    // When Rust was installed using rustup, the same toolchain's rustc
    // binary is in the same directory as the cargo binary, so we use it.
    let mut rustc = cargo.as_ref().to_owned();
    rustc.pop(); // cargo
    rustc.push(format!("rustc{}", env::consts::EXE_SUFFIX));
    if rustc.exists() {
        rustc
    } else {
        "rustc".into()
    }
}

fn rustc_version(rustc: &Path) -> Result<bool> {
    let mut cmd = cmd!(rustc, "--version", "--verbose");
    let verbose_version = cmd.read()?;
    let version = verbose_version
        .lines()
        .find_map(|line| line.strip_prefix("release: "))
        .ok_or_else(|| format_err!("unexpected version output from `{cmd}`: {verbose_version}"))?;
    let channel = version.split_once('-').map(|x| x.1).unwrap_or_default();
    let nightly = channel == "nightly" || version == "dev";
    Ok(nightly)
}

pub(crate) fn package_root(cargo: &OsStr, manifest_path: Option<&Utf8Path>) -> Result<Utf8PathBuf> {
    let package_root = if let Some(manifest_path) = manifest_path {
        manifest_path.to_owned()
    } else {
        locate_project(cargo)?.into()
    };
    Ok(package_root)
}

// https://doc.rust-lang.org/nightly/cargo/commands/cargo-locate-project.html
fn locate_project(cargo: &OsStr) -> Result<String> {
    cmd!(cargo, "locate-project", "--message-format", "plain").read()
}

// https://doc.rust-lang.org/nightly/cargo/commands/cargo-metadata.html
pub(crate) fn metadata(
    cargo: &OsStr,
    manifest_path: &Utf8Path,
) -> Result<cargo_metadata::Metadata> {
    let mut cmd = cmd!(
        cargo,
        "metadata",
        "--format-version=1",
        "--no-deps",
        "--manifest-path",
        manifest_path
    );
    let json = cmd.read()?;
    serde_json::from_str(&json).with_context(|| format!("failed to parse output from {cmd}"))
}
