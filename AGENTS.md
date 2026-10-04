# Working method
- This project is developed using atomic commits with human review: don't
  suggest humongous changes in one go; instead, interpret the user's request in
  the most narrow way possible and produce a coherent, review-friendly patch
  with a suggested commit message (one line, verb-first, terse, max 72 chars).

# Project files
- Don't ask to examine parent directories, stick to the project directory.
- Keep to an 80-column wordwrap in comments and Markdown files, where feasible.

# `AGENTS.md`
- When updating `AGENTS.md`, keep in mind that the file is meant to especially
  benefit lesser models than yourself, such as GPT-6 Sol, Terra, and Luna; but
  they may also have smaller context windows, so be terse and token-efficient.

# `TODO.md`
- Record review results & enhancement suggestions into a backlog in `TODO.md`.
- This file is meant primarily for agents and can be used as a scratchpad.
- Don't preserve mentions of already fully completed work in the file.

# `CHANGES.md`
- Only changes that affect users matter: no need to mention CI changes, etc.
- Don't include superfluous technical detail; be terse and aim for brevity.

# `README.md`
- Don't update `README.md` casually: beneficial additions require significant
  judgment and discernment, possibly beyond your capabilities. Longer and more
  detailed does *not* in fact equal better, because humans are not LLMs!
- Any documentation you might wish to add to the README likely better belongs
  as inline comments to the modules and/or types in question, where you are
  allowed elaborated length. For example, in Rust code `rustdoc` coverage of
  every public symbol is a worthy goal, but brevity and quality matter.
- The TOC structure of the README is not to be changed without asking approval.

# Rust code
- Don't ever directly read the contents of `Cargo.lock`, it can very large.
- Our current MSRV is Rust 1.97; update and enforce everywhere as needed.
- Our crates collect their default features under an `all` feature, and
  their `[features]` should start with the line `default = ["all", "std"]`.
- Our crates are meant to always be buildable with `#![no_std]` and hence
  always export an explicit `std` feature flag. Some of our lower-level crates
  may also have an explicit `alloc` feature, but for many crates it's not
  possible to do anything useful without heap allocations and they hence
  implicitly assume and omit such a feature.
- All references to `std`, `alloc`, and `core` types should always use
  qualified names or explicit, least-power imports. For example, prefer
  `core::error::Error` and `alloc::string::String` over `std` analogs.
- After making changes to a crate, as a last step run `cargo doc` on it.

# Architecture
- `jaq` is one of several intended backends; other Rust jq implementations
  may be incorporated.
- The next backend to implement is the upstream `jq` executable invoked as a
  subprocess (`jq` feature, requires `std`).
- Feature flags select the implementation behind one uniform public API.
  Keep backend abstractions private: callers should not name backend types,
  traits, type parameters, or selectors.
- Encapsulate backend-specific compilation, execution, and errors in internal
  adapters. Switching backend flags should require no caller code changes
  wherever possible.
