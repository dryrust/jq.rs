# Enhancement backlog

Refactoring constraint: `jaq` is one of several intended backends; other
Rust jq implementations may be incorporated behind the same public API.
Feature flags select the implementation, and backend abstractions stay
private. Encapsulate backend-specific types and errors so switching backends
requires no caller code changes wherever possible.

## Security: untrusted programs (e.g. web APIs)
- jq `import` feature: imports still fail with bindings or auxiliary inputs,
  as directives must precede the generated prefix/prelude. Hoist them?
- Later: opt-in API for an explicit environment map.

## Security: evaluation limits
jaq-core has no fuel/interrupt/depth hooks, and upstream declines resource
countermeasures (README "Security"; issues #467 open, #477 closed).
- jaq stack overflow aborts the host process. Overflow depths at 2 MiB/8 MiB
  stacks: program `[` nesting 1k/10k (at compile), `def f: 1+f` recursion
  1k/3k, nested output values 10k/30k. Flat `a, b, ...` lists too (#467).
- jaq, mitigation: compile and run on a worker thread with a large stack
  (e.g. 256 MiB, reserved not committed). `Filter` is `Send + Sync`, so hold
  it in `Arc`; send converted `Value`s over a rendezvous channel so the
  visitor stays on the caller; auxiliary iterators are already `Send`.
  std-only. Raises limits ~100x; callers should still cap source size.
- jaq, time/memory: not enforceable in-process. Wrapping natives for fuel
  misses pure recursion (`def f: f; f`, `repeat`), so don't ship it as a
  limit. Needs an upstream opt-in hook (per-call tick returning `Exn`;
  parser/compiler depth limit). Draft a feature request.
- Both: optional output count/size caps for `filter_json_all`.
