# Enhancement backlog

Refactoring constraint: `jaq` is one of several intended backends; other
Rust jq implementations may be incorporated behind the same public API.
Feature flags select the implementation, and backend abstractions stay
private. Encapsulate backend-specific types and errors so switching backends
requires no caller code changes wherever possible.

## Security: untrusted programs (e.g. web APIs)
- jq `import` feature: imports still fail with bindings or auxiliary inputs,
  as directives must precede the generated prefix/prelude. Hoist them?
- Later: opt-in API for an explicit environment map; evaluation limits
  (time/memory/output count) as unbounded programs are a DoS vector.
