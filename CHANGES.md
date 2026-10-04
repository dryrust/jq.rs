# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased
### Added
- `JsonFilter::with_bindings` and `set_bindings` for reusable named JSON variables
- Upstream `jq` subprocess backend, enabled with `--no-default-features --features jq`
- `JsonFilter::filter_json_visit` for incremental output and early stopping

### Fixed
- Return execution errors for `halt` and `halt_error` instead of exiting
- Report unrepresentable output numbers instead of panicking

### Changed
- Expose owned compilation diagnostics with phases and optional source spans
- Require Rust 1.97 or later
- Store execution error messages independently of backend error types

## 0.1.1 - 2026-10-04
### Added
- `JsonFilter::filter_json_all` and `filter_json_str_all` to collect all results
  and report execution errors after a result

## 0.1.0 - 2025-05-24
### Added
- `jq::JsonFilter` (requires the `jaq` feature)
