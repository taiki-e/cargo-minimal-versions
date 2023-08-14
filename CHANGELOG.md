# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [Unreleased]

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

[Unreleased]: https://github.com/taiki-e/cargo-minimal-versions/compare/v0.1.13...HEAD
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
