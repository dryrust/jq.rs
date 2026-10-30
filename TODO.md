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
- The opt-in `stacker` feature runs jaq on a 256 MiB segment, costing
  ~4.5 µs per call; off by default to keep parity with plain jaq in
  benchmarks. Persistent worker threads were slower: ~2.3 µs per output.
- Deep output values still overflow the caller when dropped or serialized
  (serde_json recursion; ~10k levels on a 2 MiB stack). Consider rejecting
  results nested deeper than a limit in `to_json`, as jq/serde_json do for
  input parsing.
- jaq, time/memory: not enforceable in-process. Wrapping natives for fuel
  misses pure recursion (`def f: f; f`, `repeat`), so don't ship it as a
  limit. Needs an upstream opt-in hook (per-call tick returning `Exn`;
  parser/compiler depth limit). Draft a feature request.
- Both: optional output count/size caps for `filter_json_all`.
