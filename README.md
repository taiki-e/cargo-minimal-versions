# cargo-minimal-versions

[![crates.io](https://img.shields.io/crates/v/cargo-minimal-versions?style=flat-square&logo=rust)](https://crates.io/crates/cargo-minimal-versions)
[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)](#license)
[![github actions](https://img.shields.io/github/actions/workflow/status/taiki-e/cargo-minimal-versions/ci.yml?branch=main&style=flat-square&logo=github)](https://github.com/taiki-e/cargo-minimal-versions/actions)

Cargo subcommand for proper use of [`-Z minimal-versions`][cargo#5657] and [`-Z direct-minimal-versions`][direct-minimal-versions].

- [Usage](#usage)
  - [--direct (-Z direct-minimal-versions)](#--direct--z-direct-minimal-versions)
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

Cargo subcommand for proper use of -Z minimal-versions and -Z direct-minimal-versions.

USAGE:
    cargo minimal-versions <CARGO_SUBCOMMAND> [OPTIONS] [CARGO_OPTIONS]

CARGO_SUBCOMMANDS:
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

**Note:** ([If cargo-minimal-versions determined that it is necessary to do so for a correct minimal versions check](#details)) cargo-minimal-versions modifies `Cargo.toml` and `Cargo.lock` while running and restores it when finished. Any changes you made to those files during running will not be preserved.

Normally, crates with `publish = false` do not need minimal versions check. You can skip these crates by using `--ignore-private` flag.

```sh
cargo minimal-versions check --workspace --ignore-private
```

If path dependencies exist, the above ways may miss the problem when you publish the crate (e.g., [tokio-rs/tokio#4376], [tokio-rs/tokio#4490]) <br>
By using `--detach-path-deps` flag, you can run minimal versions check with `path` fields removed from dependencies.

```sh
cargo minimal-versions check --workspace --ignore-private --detach-path-deps
```

`--detach-path-deps` (`--detach-path-deps=all`) flag removes all[^1] path fields by default.
By using `--detach-path-deps=skip-exact` flag, you can skip the removal of path fields in dependencies with exact version requirements (`"=<version>"`). For example, this is useful for [a pair of a proc-macro and a library that export it](https://github.com/taiki-e/pin-project/blob/df5ed4369e2c34d2111b71ef2fdd6b3621c55fa3/Cargo.toml#L32).

[^1]: To exactly, when neither version, git, nor path is specified, an error will occur, so we will remove the path field of all of dependencies for which the version or git URL is specified.

### --direct (-Z direct-minimal-versions)

If there are dependencies that are incompatible with `-Z minimum-versions`, it is also reasonable to use [`-Z direct-minimal-versions`][direct-minimal-versions], since it is hard to maintain `-Z minimum-versions` compatibility in such situations.

By using `--direct` flag, cargo-minimal-versions uses `-Z direct-minimal-versions` instead of `-Z minimal-versions`.

```sh
cargo minimal-versions check --direct
```

Note that using `-Z direct-minimal-versions` may miss some of the problems that can be found when using `-Z minimal-versions`. However, if there is a problem only in a particular version of a dependency, a problem that was missed when using `-Z minimal-versions` may be found by using `-Z direct-minimal-versions` (because the resolved dependency version is different).

## Details

Using `-Z minimal-versions` in the usual way will not work properly in many cases. [To use `cargo check` with `-Z minimal-versions` properly, you need to run at least three processes.](https://github.com/tokio-rs/tokio/pull/3131#discussion_r521621961)

> If I remember correctly, `cargo check -Z minimal-versions` doesn't really do anything. It needs to be separated into `cargo update -Z minimal-versions` and `cargo check`.
>
> Also, dev-dependencies may raise version requirements. Ideally, remove them before run `cargo update -Z minimal-versions`. (Also, note that `Cargo.lock` is actually shared within the workspace. However as far as I know, there is no workaround for this yet.)

In addition, due to cargo's feature integration, it is not correct to run `cargo check` or `cargo build` with `-p` (`--package`) or `--workspace` (`--all`) or on virtual manifest. To handle this problem correctly, you need the workspace handling provided by subcommands such as [`cargo hack`][cargo-hack].

cargo-minimal-versions addresses most of these issues and makes it easy to run cargo commands with `-Z minimal-versions`.

See [#1](https://github.com/taiki-e/cargo-minimal-versions/issues/1) and [#6](https://github.com/taiki-e/cargo-minimal-versions/issues/6) for the remaining problems.

## Installation

<!-- omit in toc -->
### Prerequisites

cargo-minimal-versions requires nightly
toolchain (to run `cargo update -Z minimal-versions` or `cargo update -Z direct-minimal-versions`) and [cargo-hack] (to run `cargo check` & `cargo build` proper):

```sh
rustup toolchain add nightly
cargo +stable install cargo-hack --locked
```

<!-- omit in toc -->
### From source

```sh
cargo +stable install cargo-minimal-versions --locked
```

Currently, installing cargo-minimal-versions requires rustc 1.70+.

cargo-minimal-versions is usually runnable with Cargo versions older than the Rust version
required for installation (e.g., `cargo +1.59 minimal-versions check`).

<!-- omit in toc -->
### From prebuilt binaries

You can download prebuilt binaries from the [Release page](https://github.com/taiki-e/cargo-minimal-versions/releases).
Prebuilt binaries are available for macOS, Linux (gnu and musl), Windows (static executable), FreeBSD, and illumos.

<details>
<summary>Example of script to download cargo-minimal-versions</summary>

```sh
# Get host target
host=$(rustc -vV | grep '^host:' | cut -d' ' -f2)
# Download binary and install to $HOME/.cargo/bin
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/taiki-e/cargo-minimal-versions/releases/latest/download/cargo-minimal-versions-$host.tar.gz | tar xzf - -C "$HOME/.cargo/bin"
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

You can install cargo-minimal-versions from the [Homebrew tap maintained by us](https://github.com/taiki-e/homebrew-tap/blob/HEAD/Formula/cargo-minimal-versions.rb) (x86_64/aarch64 macOS, x86_64/aarch64 Linux):

```sh
brew install taiki-e/tap/cargo-minimal-versions
```

<!-- omit in toc -->
### Via Scoop (Windows)

You can install cargo-minimal-versions from the [Scoop bucket maintained by us](https://github.com/taiki-e/scoop-bucket/blob/HEAD/bucket/cargo-minimal-versions.json):

```sh
scoop bucket add taiki-e https://github.com/taiki-e/scoop-bucket
scoop install cargo-minimal-versions
```

<!-- omit in toc -->
### Via cargo-binstall

You can install cargo-minimal-versions using [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall cargo-minimal-versions
```

## Related Projects

- [cargo-hack]: Cargo subcommand to provide various options useful for testing and continuous integration.
- [cargo-llvm-cov]: Cargo subcommand to easily use LLVM source-based code coverage.
- [cargo-config2]: Library to load and resolve Cargo configuration.

[cargo-config2]: https://github.com/taiki-e/cargo-config2
[cargo-hack]: https://github.com/taiki-e/cargo-hack
[cargo-llvm-cov]: https://github.com/taiki-e/cargo-llvm-cov
[cargo#5657]: https://github.com/rust-lang/cargo/issues/5657
[direct-minimal-versions]: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#direct-minimal-versions
[tokio-rs/tokio#4376]: https://github.com/tokio-rs/tokio/pull/4376
[tokio-rs/tokio#4490]: https://github.com/tokio-rs/tokio/pull/4490

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
