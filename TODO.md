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

## P2: Build, API, and regression guarantees

- [ ] Replace raw compilation debug dumps with useful diagnostics.
  `src/jaq.rs:46-65` turns loader/compiler errors into debug strings containing
  the entire source, then `JsonFilterError::Compile` debug-formats that vector
  again. Introduce owned diagnostics with phase, message, and source span,
  plus readable display formatting. Cover lexical errors (`[`), incomplete
  expressions (`1 +`), unknown functions, and unbound variables; test stable
  diagnostic information rather than upstream debug formatting.

- [ ] Add bare-metal backend coverage to CI.
  After fixing the transitive std dependency, add the bare-metal backend check
  above. Add 32-bit runtime arithmetic regressions when fixing overflow;
  compile checks alone cannot detect differing execution behavior.

## P3: Focused extensions and maintenance

- [ ] Support externally supplied variable bindings.
  `src/jaq.rs:55-57,86,122` compiles without external variable names and
  executes with `Ctx::new([])`. Add a focused constructor/builder for named
  JSON bindings so callers can reuse a compiled program with different data
  without interpolating that data into filter source. Validate binding
  names/counts and test missing bindings and repeated execution.

- [ ] Accept an explicit auxiliary input stream.
  `src/jaq.rs:85,120` always supplies an empty `RcIter`, so `input` and
  `inputs` yield nothing even when a main input value is present. Add an
  API accepting additional values for these builtins, with explicit
  exhaustion and input-error semantics. Test consumption order and keep
  the current single-JSON-value string methods' contract intact.

- [ ] Benchmark compilation, conversion, and filter reuse before optimizing.
  Add small benchmarks around `src/jaq.rs`: compile once versus per call,
  string versus `Value` input, first-result versus full collection, and
  nested/large inputs. Measure standard-library loading and value conversion
  separately enough to guide caching or allocation reductions with evidence.
