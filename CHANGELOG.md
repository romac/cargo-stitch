# Changelog

## [Unreleased]

[Unreleased]: https://github.com/romac/cargo-stitch/compare/v0.3.0...HEAD

## [0.3.0] - 2026-03-03

[0.3.0]: https://github.com/romac/cargo-stitch/compare/v0.2.1...v0.3.0

### Added

- Add support for multiple named patch sets.
- Store manifest in a temp file instead of an environment variable to work around env var size limits.
- Show tool stderr/stdout when `patch` or `ast-grep` fails.

### Changed

- Apply patches using `ast-grep` instead of `sg` to avoid a name clash on Linux.
- Use a content-hashed manifest file in `target/cargo-stitch/` to track patched state.
- Only check for required tools that are actually needed by the stitch set.
- Skip the required-tool check in wrapper mode.
- Switch to `camino` for UTF-8 path handling.
- More idiomatic serialization format for the stitch manifest.

### Fixed

- Fix race condition when the same crate is compiled concurrently.
- Skip copy and patch step when the patched directory is already up to date.

## [0.2.1] - 2026-02-25

[0.2.1]: https://github.com/romac/cargo-stitch/compare/v0.2.0...v0.2.1

### Fixed

- Fix logic for detecting `cargo-stitch` executable
- Change patched directory to `target/cargo-stitch`

## [0.2.0] - 2026-02-24

[0.2.0]: https://github.com/romac/cargo-stitch/compare/v0.1.0...v0.2.0

### Changed

- Stitch manifest is now precomputed in subcommand mode and passed to the wrapper via a JSON environment variable, reducing redundant filesystem work in the hot path.
- Workspace root is now passed to the wrapper via an environment variable instead of being re-derived.
- Improved logging output.

## [0.1.0] - 2026-02-23

[0.1.0]: https://github.com/romac/cargo-stitch/releases/tag/v0.1.0

### Added

- Initial release.

