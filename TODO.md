# Enhancement backlog

Refactoring constraint: `jaq` is one of several intended backends; other
Rust jq implementations may be incorporated behind the same public API.
Feature flags select the implementation, and backend abstractions stay
private. Encapsulate backend-specific types and errors so switching backends
requires no caller code changes wherever possible.

## Security: untrusted programs (e.g. web APIs)
- jq: `env`/`$ENV` leak host env. Unless the `env` feature is on, spawn with
  `Command::env_clear()`; also stops `~/.jq` autoload. Note: clears `TZ` for
  local-time functions. Extend `tests/environment.rs` to the jq backend.
- jq: `import "x" as $x` reads `x.json` relative to jq's cwd (any `*.json`
  reachable below it; `-L` does not help). Always wrap source as
  `.|(\n...\n)` so `import`/`include`/`module` are syntax errors; handle
  empty/comment-only programs. Opt back in via a non-default `import`
  feature (not in `all`), mirroring `env`.
- Tests: `env`, `$ENV` (jq), `import` must not reveal a set env var/file.
- Later: opt-in API for an explicit environment map; evaluation limits
  (time/memory/output count) as unbounded programs are a DoS vector.
