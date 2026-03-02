# cargo-stitch

[![Crates.io](https://img.shields.io/crates/v/cargo-stitch.svg)](https://crates.io/crates/cargo-stitch)
[![Docs.rs](https://docs.rs/cargo-stitch/badge.svg)](https://docs.rs/cargo-stitch)
[![CI](https://github.com/romac/cargo-stitch/actions/workflows/ci.yml/badge.svg)](https://github.com/romac/cargo-stitch/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/romac/cargo-stitch/graph/badge.svg?token=u0G632oVsr)](https://codecov.io/github/romac/cargo-stitch)
[![License](https://img.shields.io/crates/l/cargo-stitch.svg)](https://github.com/romac/cargo-stitch#license)

A Cargo subcommand that applies source-level patches and [ast-grep](https://ast-grep.github.io/) rules to workspace crates before compilation.

It intercepts rustc invocations via `RUSTC_WORKSPACE_WRAPPER`, copies crate sources to `target/cargo-stitch/<pkg>/`, applies patches and ast-grep rules from `stitches/<pkg>/`, then compiles the patched sources.

## Install

> [!IMPORTANT]
> Requires `patch` (usually preinstalled) and [`ast-grep`](https://ast-grep.github.io/guide/quick-start.html) (`sg`) if using ast-grep rules.

**Using [`cargo binstall`](https://github.com/cargo-bins/cargo-binstall)** (recommended, downloads prebuilt binaries):

```
cargo binstall cargo-stitch
```

**Using `cargo install`** (builds from source):

```
cargo install cargo-stitch
```

**From [GitHub Releases](https://github.com/romac/cargo-stitch/releases)**: download a prebuilt binary for your platform, extract, and place it on your `PATH`.

**From source**:

```
git clone https://github.com/romac/cargo-stitch
cd cargo-stitch
cargo install --path .
```


## Usage

```
cargo stitch build
cargo stitch test
cargo stitch check
# any cargo subcommand works
```

## Stitch files

Place stitch files in `stitches/<crate-name>/` at the workspace root:

- **`.patch`** -- unified diff format, applied with `patch -p1`
- **`.yaml` / `.yml`** -- ast-grep rule files, applied with `sg scan -r <rule> --update-all`

All stitch files are applied in filename order regardless of type. Use numeric prefixes for ordering:

```
stitches/
  some-crate/
    001-fix-thing.patch
    002-rename-fn.yaml
```

If no `stitches/<crate-name>/` directory exists for a crate, it compiles normally.

## Acknowledgements

Inspired by [cargo-fixup](https://github.com/cecton/cargo-fixup).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
