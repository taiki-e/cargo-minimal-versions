# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

Releases may yanked if there is a security bug, a soundness bug, or a regression.

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [Unreleased]

- Update `toml_edit` to 0.24. This includes TOML 1.1 parse support.

## [0.1.33] - 2025-09-19

- Fix bug casing ["feature `...` includes `..`, but `..` is not a dependency" error](https://github.com/taiki-e/cargo-minimal-versions/issues/34).

## [0.1.32] - 2025-09-07

- Distribute prebuilt binaries for powerpc64le/riscv64gc/s390x Linux.

## [0.1.31] - 2025-07-11

- Update `toml_edit` to 0.23.

## [0.1.30] - 2025-02-11

- Performance improvements.

- Documentation improvements.

## [0.1.29] - 2024-10-05

- Work around "lock file version `4` was found, but this version of Cargo does not understand this lock file" error related to the recent nightly Cargo change. ([#31](https://github.com/taiki-e/cargo-minimal-versions/issues/31))

- Disable quick-install fallback of cargo-binstall.

## [0.1.28] - 2024-07-15

- Distribute prebuilt binary for x86_64 illumos.

- Always exit with 1 on SIGINT/SIGTERM/SIGHUP. Previously, it sometimes exited with 0, but this sometimes worked badly with CI systems that attempted to terminate processes in SIGINT during resource usage problems.

## [0.1.27] - 2024-03-19

- Improve support for environments without rustup or nightly toolchain installed. Previously, an explicit `RUSTC_BOOTSTRAP=1` was required if rustc is not nightly but it is no longer required.

## [0.1.26] - 2024-03-10

- Pin `ctrlc` to fix [build error on macOS](https://github.com/Detegr/rust-ctrlc/pull/116).

## [0.1.25] - 2024-02-10

- Update `toml_edit` to 0.22.

## [0.1.24] - 2024-01-24

- Fix "No such file or directory" error when `--no-private` flag is used with the workspace that the `members` field contains glob.

## [0.1.23] - 2023-12-16

- Remove dependency on `is-terminal`.

## [0.1.22] - 2023-12-05

- Update `toml_edit` to 0.21.

## [0.1.21] - 2023-10-27

- Add `--direct` flag to use [`-Z direct-minimal-versions`](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#direct-minimal-versions) instead of `-Z minimal-versions`. ([#25](https://github.com/taiki-e/cargo-minimal-versions/pull/25))

## [0.1.20] - 2023-10-22

- Add `--detach-path-deps` flag to run minimal versions check with `path` fields removed from dependencies. ([#4](https://github.com/taiki-e/cargo-minimal-versions/pull/4))

## [0.1.19] - 2023-09-11

- Remove dependency on `slab`, `shell-escape`, and `fs-err`.

## [0.1.18] - 2023-09-10

- Fix regression on `--no-private` flag with virtual workspace, introduced in 0.1.17.

## [0.1.17] - 2023-09-09

- Improve support for very old Cargo (pre-1.39).

- Remove dependency on `cargo_metadata`.

## [0.1.16] - 2023-08-28

- Fix bug in `--ignore-private`/`--no-private` flag on Windows.

## [0.1.15] - 2023-08-28

- Improve the behavior of `--ignore-private` flag to prevent private crates from affecting lockfile and metadata.

  This fixes some false negatives.

- Add `--no-private` flag as an alias of `--ignore-private` flag.

## [0.1.14] - 2023-08-14

- Allow nightly to be specified by setting `RUSTC_BOOTSTRAP=1`, the same as for rustc and cargo.

## [0.1.13] - 2023-07-28

- Update `cargo_metadata` to 0.17.

## [0.1.12] - 2023-04-15

- Fix version detection with dev build.

- Update `toml_edit` to 0.19.

## [0.1.11] - 2023-01-24

- Update `toml_edit` to 0.18.

- Update `lexopt` to 0.3

## [0.1.10] - 2023-01-11

- Distribute prebuilt macOS universal binary.

- Distribute prebuilt binary for x86_64 FreeBSD.

- Update `toml_edit` to 0.17.

## [0.1.9] - 2022-12-25

- Update `toml_edit` to 0.16.

## [0.1.8] - 2022-11-27

- Replace `atty` with `is-terminal`. ([#11](https://github.com/taiki-e/cargo-minimal-versions/pull/11))

## [0.1.7] - 2022-10-25

- Work around a rustup bug ([rust-lang/rustup#3036](https://github.com/rust-lang/rustup/issues/3036)) on Windows.

## [0.1.6] - 2022-10-25

- Update `toml_edit` to 0.15.

  This increases the rustc version required to build cargo-minimal-versions. (rustc 1.56+ -> 1.60+)
  The cargo/rustc version required to run cargo-minimal-versions remains unchanged.

- Distribute prebuilt binaries for AArch64 Windows.

## [0.1.5] - 2022-07-08

- Add metadata for cargo binstall.

## [0.1.4] - 2022-06-02

- Distribute prebuilt binaries for AArch64 macOS. ([#7](https://github.com/taiki-e/cargo-minimal-versions/pull/7))

## [0.1.3] - 2022-02-05

- Warn when unrecognized subcommand is passed. ([#3](https://github.com/taiki-e/cargo-minimal-versions/pull/3))

## [0.1.2] - 2022-01-21

- Distribute prebuilt binaries for AArch64 Linux (gnu and musl).

## [0.1.1] - 2022-01-05

- Respect `--manifest-path` option.

## [0.1.0] - 2021-12-28

Initial release

[Unreleased]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.33...HEAD
[0.1.33]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.32...v0.1.33
[0.1.32]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.31...v0.1.32
[0.1.31]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.30...v0.1.31
[0.1.30]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.29...v0.1.30
[0.1.29]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.28...v0.1.29
[0.1.28]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.27...v0.1.28
[0.1.27]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.26...v0.1.27
[0.1.26]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.25...v0.1.26
[0.1.25]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.24...v0.1.25
[0.1.24]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.23...v0.1.24
[0.1.23]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.22...v0.1.23
[0.1.22]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.21...v0.1.22
[0.1.21]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.20...v0.1.21
[0.1.20]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.19...v0.1.20
[0.1.19]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.18...v0.1.19
[0.1.18]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.17...v0.1.18
[0.1.17]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.16...v0.1.17
[0.1.16]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.15...v0.1.16
[0.1.15]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.14...v0.1.15
[0.1.14]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.13...v0.1.14
[0.1.13]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.12...v0.1.13
[0.1.12]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/taiki-e/cargo-minimal-versions/releases/tag/v0.1.0
