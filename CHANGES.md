# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `JsonFilter::set_timeout` and `JsonFilterError::Timeout` to stop jq backend
  evaluations at a deadline
- `stacker` feature to run jaq on a large stack, so deeply nested programs
  and values no longer crash the host process at modest depths
### Changed
- Mark `JsonFilterError` as `#[non_exhaustive]`
### Security
- Reject jaq results nested more than 127 levels deep, as the jq backend
  already did, so callers cannot overflow their stacks on deep results

## [0.3.1] - 2026-10-30
### Added
- `log` feature to forward jaq's `stderr` and `debug` messages to `log`
### Security
- Discard jaq's `stderr` and `debug` messages unless the `log` feature is
  enabled, as programs could write arbitrary text into the host's logs

## [0.3.0] - 2026-10-30
### Added
- `env` feature to expose host environment variables to programs
- `import` feature to let jq programs load modules and JSON data files
### Security
- Return an empty object from `env` unless the `env` feature is enabled
- Run jq with an empty environment unless the `env` feature is enabled,
  which also disables `~/.jq` and makes local time UTC
- Reject jq `import`, `include`, and `module` unless the `import` feature is
  enabled, as imports could read JSON files from the working directory

## [0.2.0] - 2026-10-05
### Added
- Upstream `jq` subprocess backend, enabled with `--no-default-features --features jq`
- Fallible auxiliary input streams for first, collected, and visited results
- `JsonFilter::with_bindings` and `set_bindings` for reusable named JSON variables
- `JsonFilter::filter_json_visit` for incremental output and early stopping
### Changed
- Bump the MSRV to Rust 1.97
- Reject non-JSON backend values with `JsonFilterError::OutputValue`
- Expose owned compilation diagnostics with phases and optional source spans
- Store execution error messages independently of backend error types
### Fixed
- Support the jaq backend on no-std targets
- Return errors for invalid Unicode code points instead of panicking
- Promote overflowing integer arithmetic instead of wrapping or panicking
- Report `input` exhaustion as an execution error on both backends
- Reject unsupported data imports during compilation
- Return execution errors for `halt` and `halt_error` instead of exiting
- Report unrepresentable output numbers instead of panicking

## [0.1.1] - 2026-10-04
### Added
- `JsonFilter::filter_json_all` and `filter_json_str_all` to collect all results
  and report execution errors after a result

## [0.1.0] - 2025-05-24
### Added
- `jq::JsonFilter` (requires the `jaq` feature)

[Unreleased]: https://github.com/dryrust/jq.rs/compare/0.3.1...HEAD
[0.3.1]: https://github.com/dryrust/jq.rs/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/dryrust/jq.rs/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/dryrust/jq.rs/compare/0.1.1...0.2.0
[0.1.1]: https://github.com/dryrust/jq.rs/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/dryrust/jq.rs/releases/tag/0.1.0
