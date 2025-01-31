// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(not(miri))] // Miri doesn't support file with non-default mode: https://github.com/rust-lang/miri/pull/2720

use std::{ffi::OsStr, path::Path, process::Command};

use test_helper::cli::CommandExt as _;

fn cargo_minimal_versions<O: AsRef<OsStr>>(args: impl AsRef<[O]>) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-minimal-versions"));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd.arg("minimal-versions");
    cmd.args(args.as_ref());
    cmd
}

#[test]
fn help() {
    let short = cargo_minimal_versions(["-h"]).assert_success();
    let long = cargo_minimal_versions(["--help"]).assert_success();
    assert_eq!(short.stdout, long.stdout);
}

#[test]
fn version() {
    let expected = &format!("cargo-minimal-versions {}", env!("CARGO_PKG_VERSION"));
    cargo_minimal_versions(["-V"]).assert_success().stdout_eq(expected);
    cargo_minimal_versions(["--version"]).assert_success().stdout_eq(expected);
}

#[test]
fn update_readme() {
    let new = cargo_minimal_versions(["--help"]).assert_success().stdout;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let command = "cargo minimal-versions --help";
    test_helper::doc::sync_command_output_to_markdown(path, "readme-long-help", command, new);
}
