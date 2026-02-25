# Changelog

## [Unreleased]

[Unreleased]: https://github.com/romac/cargo-stitch/compare/v0.2.1...HEAD

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

