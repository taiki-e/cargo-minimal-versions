// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{env, ffi::OsStr, path::PathBuf};

use anyhow::{Result, format_err};

use crate::{metadata, process::ProcessBuilder};

pub(crate) struct Workspace {
    pub(crate) metadata: metadata::Metadata,
    cargo: PathBuf,
    cargo_mode: CargoMode,
}

enum CargoMode {
    Nightly,
    StableHasUnstableOption,
    StableNoUnstableOption,
}

impl Workspace {
    pub(crate) fn new(manifest_path: Option<&str>, direct: bool) -> Result<Self> {
        let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let cargo_version = cargo_version(&cargo)?;

        let metadata = metadata::Metadata::new(manifest_path, &cargo, cargo_version.minor)?;

        let cargo_mode = if cargo_version.nightly {
            CargoMode::Nightly
        } else if !direct
            || cmd!(&cargo, "-Z", "help")
                .env("RUSTC_BOOTSTRAP", "1")
                .read()
                .unwrap_or_default()
                .contains("direct-minimal-versions")
            || cmd!("rustup", "run", "nightly", "cargo", "--version").run_with_output().is_err()
        {
            // Favor `RUSTC_BOOTSTRAP=1 cargo update -Z ...` over `rustup run nightly cargo update -Z ...`
            // when the corresponding unstable option is available on the current toolchain version.
            CargoMode::StableHasUnstableOption
        } else {
            CargoMode::StableNoUnstableOption
        };

        Ok(Self { cargo: cargo.into(), cargo_mode, metadata })
    }

    pub(crate) fn cargo(&self) -> ProcessBuilder {
        cmd!(&self.cargo)
    }

    // Used for `cargo update -Z minimal-versions` / `cargo update -Z direct-minimal-versions`
    pub(crate) fn cargo_nightly(&self) -> ProcessBuilder {
        match self.cargo_mode {
            CargoMode::Nightly => self.cargo(),
            CargoMode::StableHasUnstableOption => {
                let mut cargo = self.cargo();
                cargo.env("RUSTC_BOOTSTRAP", "1");
                cargo
            }
            // Do not use `cargo +nightly` due to a rustup bug: https://github.com/rust-lang/rustup/issues/3036
            CargoMode::StableNoUnstableOption => cmd!("rustup", "run", "nightly", "cargo"),
        }
    }
}

fn cargo_version(cargo: &OsStr) -> Result<CargoVersion> {
    // Use verbose version output because the packagers add extra strings to the normal version output.
    let mut cmd = cmd!(cargo, "-vV");
    let verbose_version = cmd.read()?;
    CargoVersion::parse(&verbose_version)
        .ok_or_else(|| format_err!("unexpected version output from {cmd}: {verbose_version}"))
}

struct CargoVersion {
    minor: u32,
    nightly: bool,
}

impl CargoVersion {
    fn parse(verbose_version: &str) -> Option<Self> {
        let release = verbose_version.lines().find_map(|line| line.strip_prefix("release: "))?;
        let (version, channel) = release.split_once('-').unwrap_or((release, ""));
        let mut digits = version.splitn(3, '.');
        let major = digits.next()?;
        if major != "1" {
            return None;
        }
        let minor = digits.next()?.parse::<u32>().ok()?;
        let _patch = digits.next()?.parse::<u32>().ok()?;
        let nightly = channel == "nightly"
            || channel == "dev"
            || env::var_os("RUSTC_BOOTSTRAP").unwrap_or_default() == "1";

        Some(Self { minor, nightly })
    }
}
