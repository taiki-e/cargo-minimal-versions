use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{format_err, Result};

use crate::{metadata, process::ProcessBuilder};

pub(crate) struct Workspace {
    pub(crate) metadata: metadata::Metadata,
    cargo: PathBuf,
    nightly: bool,
}

impl Workspace {
    pub(crate) fn new(manifest_path: Option<&str>) -> Result<Self> {
        let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let rustc = rustc_path(&cargo);
        let rustc_version = rustc_version(&rustc)?;

        let metadata = metadata::Metadata::new(manifest_path, &cargo, rustc_version.minor)?;

        Ok(Self { cargo: cargo.into(), nightly: rustc_version.nightly, metadata })
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

struct RustcVersion {
    minor: u32,
    nightly: bool,
}

fn rustc_version(rustc: &Path) -> Result<RustcVersion> {
    let mut cmd = cmd!(rustc, "--version", "--verbose");
    let verbose_version = cmd.read()?;
    let release = verbose_version
        .lines()
        .find_map(|line| line.strip_prefix("release: "))
        .ok_or_else(|| format_err!("unexpected version output from `{cmd}`: {verbose_version}"))?;
    let (version, channel) = release.split_once('-').unwrap_or((release, ""));
    let mut digits = version.splitn(3, '.');
    let minor = (|| {
        let major = digits.next()?.parse::<u32>().ok()?;
        if major != 1 {
            return None;
        }
        let minor = digits.next()?.parse::<u32>().ok()?;
        let _patch = digits.next().unwrap_or("0").parse::<u32>().ok()?;
        Some(minor)
    })()
    .ok_or_else(|| format_err!("unable to determine rustc version"))?;
    let nightly = channel == "nightly"
        || channel == "dev"
        || env::var("RUSTC_BOOTSTRAP").ok().as_deref() == Some("1");
    Ok(RustcVersion { minor, nightly })
}
