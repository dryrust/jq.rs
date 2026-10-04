# Enhancement backlog

Review baseline: `jq` 0.1.1, 2026-10-04. Runtime reproductions use Rust
1.98.1 on aarch64 macOS and the locked jaq-core 2.2.0, jaq-json 1.1.2, and
jaq-std 2.1.1 dependencies, with default features unless otherwise noted.

Refactoring constraint: `jaq` is one of several intended backends; other
Rust jq implementations may be incorporated behind the same public API.
Feature flags select the implementation, and backend abstractions stay
private. Encapsulate backend-specific types and errors so switching backends
requires no caller code changes wherever possible.

## P1: Correctness and portability

- [ ] Eliminate profile-dependent integer overflow in the backend.
  Through `src/jaq.rs` evaluation, `9223372036854775807 + 1` panics in debug
  builds but returns `-9223372036854775808` in release builds. Fix or upgrade
  the jaq-json arithmetic implementation to promote or report overflow
  consistently. Add boundary regressions for arithmetic in both profiles
  and account for the backend's machine-sized integers on 32-bit targets.

- [ ] Make the `jaq` feature genuinely usable without `std`.
  In `Cargo.toml:28,40-45`, disabling direct dependency defaults does not
  disable transitive defaults: jaq-json enables jaq-std's default features,
  and the graph also enables jaq-core/std and std-dependent crates such as
  once_cell and regex-lite. Fix or upgrade the transitive feature wiring.
  `cargo check --no-default-features --features jaq` passes on the host but
  fails with E0463 when adding `--target thumbv7em-none-eabihf`; the same
  target builds with no features. Require the functional backend to pass
  this bare-metal check, rather than checking only the empty API surface.

  Upgrade note: jaq-json 1.1.3 still enables transitive defaults. Current
  upstream jaq-json 2.x changes the value and backend APIs, so portability and
  arithmetic fixes need a coordinated dependency migration.

## P2: Build, API, and regression guarantees

- [ ] Add bare-metal backend coverage to CI.
  After fixing the transitive std dependency, add the bare-metal backend check
  above. Add 32-bit runtime arithmetic regressions when fixing overflow;
  compile checks alone cannot detect differing execution behavior.

## P3: Focused extensions and maintenance

- [ ] Accept an explicit auxiliary input stream.
  Add fallible input handling to the jq subprocess adapter, expose it through
  the common API, and test consumption order and error semantics on both
  backends. Keep single-JSON-value string methods' existing parsing contract.
