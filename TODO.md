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
  Audit negation, remainder, and division boundaries in jaq-json 2.x. Add
  regressions in both profiles and account for 32-bit machine-sized integers.

- [ ] Verify no-std capability boundaries.
  Test core filtering and float rounding without std, and document optional
  standard-library functions that require std. Compile a consumer using the
  public API on bare metal, rather than checking only the library itself.

- [ ] Define JSON-output compatibility for jaq-json 2.x.
  Test rejection of binary strings, invalid UTF-8, and non-string object keys.
  Cover computed big integers with consumer arbitrary precision enabled.

## P2: Build, API, and regression guarantees

- [ ] Add bare-metal backend coverage to CI.
  After fixing the transitive std dependency, add the bare-metal backend check
  above. Add 32-bit runtime arithmetic regressions when fixing overflow;
  compile checks alone cannot detect differing execution behavior.
