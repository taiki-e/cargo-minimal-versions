use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{format_err, Context as _, Result};

use crate::process::ProcessBuilder;

pub(crate) struct Workspace {
    pub(crate) metadata: cargo_metadata::Metadata,
    cargo: PathBuf,
    nightly: bool,
}

impl Workspace {
    pub(crate) fn new(manifest_path: Option<&str>) -> Result<Self> {
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
            // Do not use `cargo +nightly` due to a rustup bug: https://github.com/rust-lang/rustup/issues/3036
            cmd!("rustup", "run", "nightly", "cargo")
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
    let nightly = channel == "nightly"
        || channel == "dev"
        || env::var("RUSTC_BOOTSTRAP").ok().as_deref() == Some("1");
    Ok(nightly)
}

fn package_root(cargo: &OsStr, manifest_path: Option<&str>) -> Result<String> {
    let package_root = if let Some(manifest_path) = manifest_path {
        manifest_path.to_owned()
    } else {
        locate_project(cargo)?
    };
    Ok(package_root)
}

// https://doc.rust-lang.org/nightly/cargo/commands/cargo-locate-project.html
fn locate_project(cargo: &OsStr) -> Result<String> {
    cmd!(cargo, "locate-project", "--message-format", "plain").read()
}

// https://doc.rust-lang.org/nightly/cargo/commands/cargo-metadata.html
fn metadata(cargo: &OsStr, manifest_path: &str) -> Result<cargo_metadata::Metadata> {
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
