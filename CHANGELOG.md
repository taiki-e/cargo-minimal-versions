# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [Unreleased]

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

- Distribute prebuilt binaries for aarch64 Windows.

## [0.1.5] - 2022-07-08

- Add metadata for cargo binstall.

## [0.1.4] - 2022-06-02

- Distribute prebuilt binaries for aarch64 macOS. ([#7](https://github.com/taiki-e/cargo-minimal-versions/pull/7))

## [0.1.3] - 2022-02-05

- Warn when unrecognized subcommand is passed. ([#3](https://github.com/taiki-e/cargo-minimal-versions/pull/3))

## [0.1.2] - 2022-01-21

- Distribute prebuilt binaries for aarch64 Linux (gnu and musl).

## [0.1.1] - 2022-01-05

- Respect `--manifest-path` option.

## [0.1.0] - 2021-12-28

Initial release

[Unreleased]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.20...HEAD
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
