# cargo-stitch Example

This directory contains a multi-crate workspace that demonstrates how to use cargo-stitch to apply patches and ast-grep rules to workspace dependencies before compilation.

## Workspace Structure

```
example/
├── Cargo.toml           # Workspace manifest
├── app/                 # Application crate that depends on config
│   ├── Cargo.toml
│   └── src/main.rs
├── config/              # Library crate (original, unpatched source)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── parser.rs
└── stitches/
    └── config/          # Stitch files for the config crate
        ├── 001-add-error-handling.patch
        ├── 002-unwrap-to-expect.yaml
        ├── 003-add-display-and-iter.patch
        └── 004-hashmap-with-capacity.yaml
```

## What the Patches Do

### 001-add-error-handling.patch (multi-file, multi-hunk)

A unified diff that modifies both `lib.rs` and `parser.rs`:

- Adds `ConfigError` enum with `Io` and `Parse` variants
- Implements `Display` and `Error` for `ConfigError`
- Changes `Config::load()` to return `Result<Config, ConfigError>` instead of panicking
- Adds `Default` impl for `Config`
- Adds `is_empty()` method
- Adds `Clone` derive to `ParseError`

### 002-unwrap-to-expect.yaml (ast-grep rule)

Transforms all `.unwrap()` calls to `.expect("value expected")`:

```yaml
id: unwrap-to-expect
language: Rust
rule:
  pattern: $EXPR.unwrap()
fix: $EXPR.expect("value expected")
```

### 003-add-display-and-iter.patch

- Adds `Display` impl for `Value` enum
- Adds `iter()` method to `Config` for iterating over entries
- Re-exports `EntriesIter` type

### 004-hashmap-with-capacity.yaml (ast-grep rule)

Changes `HashMap::new()` to `HashMap::with_capacity(16)` for better initial allocation:

```yaml
id: hashmap-with-capacity
language: Rust
rule:
  pattern: HashMap::new()
fix: HashMap::with_capacity(16)
```

## Running the Example

From the `example/` directory:

```bash
# Build with patches applied
cargo stitch build

# Run the application
cargo stitch run

# Check the patched source
cat target/patched-crates/config/src/lib.rs
```

## How It Works

1. `cargo stitch build` invokes cargo with `RUSTC_WORKSPACE_WRAPPER` set to cargo-stitch
2. When cargo compiles the `config` crate, cargo-stitch intercepts the rustc call
3. cargo-stitch copies `config/` to `target/patched-crates/config/`
4. Stitch files are applied in filename order:
   - `.patch` files via `patch -p1`
   - `.yaml`/`.yml` files via `sg scan --update-all`
5. rustc is invoked with paths rewritten to point at the patched sources
