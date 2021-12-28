use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};

use crate::process::ProcessBuilder;

pub(crate) struct Context {
    cargo: PathBuf,
    nightly: bool,
    pub(crate) metadata: cargo_metadata::Metadata,
}

impl Context {
    pub(crate) fn new(manifest_path: Option<&Utf8Path>) -> Result<Self> {
        let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let version = cmd!(&cargo, "--version").read()?;
        let nightly = version.contains("nightly")
            || version.contains("dev")
            // Check if `rustc -Z help` succeeds, to support custom built toolchains
            // with nightly features enabled.
            || cmd!(rustc_path(&cargo), "-Z", "help").run_with_output().is_ok();
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
        "--format-version",
        "1",
        "--no-deps",
        "--manifest-path",
        manifest_path
    );
    let json = match cmd.read() {
        Ok(json) => json,
        Err(e) => {
            // Retry with stable cargo because if workspace member has
            // a dependency that requires newer cargo features, `cargo metadata`
            // with older cargo may fail.
            cmd = cmd!(
                "cargo",
                "+stable",
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--manifest-path",
                manifest_path
            );
            match cmd.read() {
                Ok(json) => json,
                Err(_e) => return Err(e),
            }
        }
    };
    serde_json::from_str(&json).with_context(|| format!("failed to parse output from {}", cmd))
}
