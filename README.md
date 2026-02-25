# cargo-stitch

A Cargo subcommand that applies source-level patches and [ast-grep](https://ast-grep.github.io/) rules to workspace crates before compilation.

It intercepts rustc invocations via `RUSTC_WORKSPACE_WRAPPER`, copies crate sources to `target/cargo-stitch/<pkg>/`, applies patches and ast-grep rules from `stitches/<pkg>/`, then compiles the patched sources.

## Install

```
cargo install --path .
```

Requires `patch` (usually preinstalled) and [`ast-grep`](https://ast-grep.github.io/guide/quick-start.html) (`sg`) if using ast-grep rules.

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
