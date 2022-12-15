# cargo-minimal-versions

[![crates.io](https://img.shields.io/crates/v/cargo-minimal-versions?style=flat-square&logo=rust)](https://crates.io/crates/cargo-minimal-versions)
[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)](#license)
[![rustc](https://img.shields.io/badge/rustc-1.60+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![build status](https://img.shields.io/github/actions/workflow/status/taiki-e/cargo-minimal-versions/ci.yml?branch=main&style=flat-square&logo=github)](https://github.com/taiki-e/cargo-minimal-versions/actions)

Cargo subcommand for proper use of [`-Z minimal-versions`][cargo#5657].

- [Usage](#usage)
- [Details](#details)
- [Installation](#installation)
- [Related Projects](#related-projects)
- [License](#license)

## Usage

<details>
<summary>Click to show a complete list of options</summary>

<!-- readme-long-help:start -->
```console
$ cargo minimal-versions --help
cargo-minimal-versions

Cargo subcommand for proper use of -Z minimal-versions.

USAGE:
    cargo minimal-versions <SUBCOMMAND> [CARGO_OPTIONS]

SUBCOMMANDS:
    build
    check
    test
    ...
```
<!-- readme-long-help:end -->

</details>

To check all crates with minimal version dependencies:

```sh
cargo minimal-versions check --workspace
```

Normally, crates with `publish = false` do not need minimal version check. You can skip these crates by using `--ignore-private` flag.

```sh
cargo minimal-versions check --workspace --ignore-private
```

## Details

Using `-Z minimal-versions` in the usual way will not work properly in many cases. [To use `cargo check` with `-Z minimal-versions` properly, you need to run at least three processes.](https://github.com/tokio-rs/tokio/pull/3131#discussion_r521621961)

> If I remember correctly, `cargo check -Z minimal-versions` doesn't really do anything. It needs to be separated into `cargo update -Z minimal-versions` and `cargo check`.
>
> Also, dev-dependencies may raise version requirements. Ideally, remove them before run `cargo update -Z minimal-versions`. (Also, note that `Cargo.lock` is actually shared within the workspace. However as far as I know, there is no workaround for this yet.)

In addition, due to cargo's feature integration, it is not correct to run `cargo check` or `cargo build` with `-p` (`--package`) or `--workspace` (`--all`) or on virtual manifest. To handle this problem correctly, you need the workspace handling provided by subcommands such as [`cargo hack`][cargo-hack].

cargo-minimal-versions addresses most of these issues and makes it easy to run cargo commands with `-Z minimal-versions`.

See [#1](https://github.com/taiki-e/cargo-minimal-versions/issues/1) for the remaining problem.

## Installation

<!-- omit in toc -->
### Prerequisites

cargo-minimal-versions requires nightly
toolchain (to run `cargo update -Z minimal-versions`) and [cargo-hack] (to run `cargo check` & `cargo build` proper):

```sh
rustup toolchain add nightly
cargo +stable install cargo-hack
```

<!-- omit in toc -->
### From source

```sh
cargo +stable install cargo-minimal-versions
```

*Compiler support: requires rustc 1.60+*

<!-- TODO: test
cargo-minimal-versions is usually runnable with Cargo versions older than the Rust version
required for installation (e.g., `cargo +1.31 hack check`). Currently, to run
cargo-minimal-versions requires Cargo 1.26+.
-->

<!-- omit in toc -->
### From prebuilt binaries

You can download prebuilt binaries from the [Release page](https://github.com/taiki-e/cargo-minimal-versions/releases).
Prebuilt binaries are available for macOS, Linux (gnu and musl), and Windows (static executable).

<details>
<summary>Example of script to download cargo-minimal-versions</summary>

```sh
# Get host target
host=$(rustc -Vv | grep host | sed 's/host: //')
# Download binary and install to $HOME/.cargo/bin
curl -LsSf https://github.com/taiki-e/cargo-minimal-versions/releases/latest/download/cargo-minimal-versions-$host.tar.gz | tar xzf - -C $HOME/.cargo/bin
```

</details>

<!-- omit in toc -->
### On GitHub Actions

You can use [taiki-e/install-action](https://github.com/taiki-e/install-action) to install prebuilt binaries on Linux, macOS, and Windows.
This makes the installation faster and may avoid the impact of [problems caused by upstream changes](https://github.com/tokio-rs/bytes/issues/506).

```yaml
- uses: taiki-e/install-action@cargo-hack
- uses: taiki-e/install-action@cargo-minimal-versions
```

<!-- omit in toc -->
### Via Homebrew

You can install [cargo-minimal-versions using Homebrew tap on macOS and Linux](https://github.com/taiki-e/homebrew-tap/blob/main/Formula/cargo-minimal-versions.rb):

```sh
brew install taiki-e/tap/cargo-minimal-versions
```

<!-- omit in toc -->
### Via Scoop (Windows)

You can install [cargo-minimal-versions using Scoop](https://github.com/taiki-e/scoop-bucket/blob/HEAD/bucket/cargo-minimal-versions.json):

```sh
scoop bucket add taiki-e https://github.com/taiki-e/scoop-bucket
scoop install cargo-minimal-versions
```

<!-- omit in toc -->
### Via cargo-binstall

You can install cargo-minimal-versions using [cargo-binstall](https://github.com/ryankurte/cargo-binstall):

```sh
cargo binstall cargo-minimal-versions
```

## Related Projects

- [cargo-hack]: Cargo subcommand to provide various options useful for testing and continuous integration.
- [cargo-llvm-cov]: Cargo subcommand to easily use LLVM source-based code coverage.

[cargo-hack]: https://github.com/taiki-e/cargo-hack
[cargo-llvm-cov]: https://github.com/taiki-e/cargo-llvm-cov
[cargo#5657]: https://github.com/rust-lang/cargo/issues/5657

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
