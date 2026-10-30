# jq.rs

[![License](https://img.shields.io/badge/license-Public%20Domain-blue.svg)](https://unlicense.org)
[![Compatibility](https://img.shields.io/badge/rust-1.97%2B-blue)](https://endoflife.date/rust)
[![Package on Crates.io](https://img.shields.io/crates/v/jq)](https://crates.io/crates/jq)
[![Documentation](https://img.shields.io/docsrs/jq?label=docs.rs)](https://docs.rs/jq)

**jq.rs brings [jq]-style JSON filtering to Rust.**

<sub>

[[Features](#-features)] |
[[Prerequisites](#%EF%B8%8F-prerequisites)] |
[[Installation](#%EF%B8%8F-installation)] |
[[Examples](#-examples)] |
[[Reference](#-reference)] |
[[Development](#%E2%80%8D-development)]

</sub>

<br/>

## ✨ Features

- Compile filters once and reuse them with the in-process [jaq] backend.
- Work with JSON strings or [`serde_json::Value`] inputs.
- Return the first result, collect all results, or visit results incrementally.
- Bind named JSON variables and supply auxiliary inputs to `input` and `inputs`.
- Choose [jaq] or the upstream [jq] executable through Cargo feature flags.
- Supports `no_std` with heap allocation.
- Cuts red tape: 100% free and unencumbered public domain software.

## 🛠️ Prerequisites

- [Rust] 1.97+ (2024 edition)
- A [jq] executable in the `PATH` (only when using the subprocess backend)

## ⬇️ Installation

Install the latest release from [Crates.io]:

```bash
cargo add jq serde_json
```

For the development version documented here, including named bindings and the
subprocess backend:

```bash
cargo add jq --git https://github.com/dryrust/jq.rs
cargo add serde_json
```

## 👉 Examples

### Filtering JSON

Parse a filter once and reuse it. Choose the first result or collect them all:

```rust
use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

fn main() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = ".items[] | .name".parse()?;
    let input = r#"{"items": [{"name": "Ada"}, {"name": "Lin"}]}"#;

    assert_eq!(
        filter.filter_json_str(input)?,
        json!("Ada"),
    );
    assert_eq!(
        filter.filter_json_str_all(input)?,
        [json!("Ada"), json!("Lin")],
    );
    Ok(())
}
```

### Binding Variables

Pass values as named variables without interpolating them into filter source:

```rust
use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

fn main() -> Result<(), JsonFilterError> {
    let mut filter = JsonFilter::with_bindings(
        ".[] | select(. >= $minimum)",
        [("minimum", json!(10))],
    )?;
    assert_eq!(
        filter.filter_json_all(json!([5, 12, 20]))?,
        [json!(12), json!(20)],
    );

    filter.set_bindings([("minimum", json!(15))])?;
    assert_eq!(filter.filter_json_all(json!([5, 12, 20]))?, [json!(20)]);
    Ok(())
}
```

## 📚 Reference

### Rust API

[`JsonFilter`] compiles and evaluates filters; [`JsonFilterError`] reports
failures. See the [API documentation] for method contracts and backend behavior,
or run `cargo doc --open` for the development version.

### Feature Flags

| Feature | Purpose |
| --- | --- |
| `jaq` | In-process Rust backend. |
| `jq` | Upstream jq subprocess backend; enables `std`. |
| `std` | Standard-library support and jaq's optional functions. |
| `env` | Exposes host environment variables to `env`; enables `std`. |
| `import` | Lets the jq backend load modules and JSON data files. |
| `log` | Forwards jaq's `stderr` and `debug` messages to [`log`]. |
| `all` | Enables both backends. |

Defaults enable `all` and `std`. `env` and `import` are opt-in, as they
expose secrets in environment variables and files; never enable them for
untrusted programs. `jaq` takes precedence when both backends are
enabled. To select the subprocess backend, set `default-features = false` and
`features = ["jq"]` on the dependency. For `no_std` with heap allocation, use
`default-features = false` and `features = ["jaq"]` instead.

Both backends use the same Rust API, but their jq language behavior can differ.

## 👨‍💻 Development

```bash
git clone https://github.com/dryrust/jq.rs.git
```

<sup><sub>This README was generated with [Readmer].</sub></sup>

---

[![Share on X](https://img.shields.io/badge/share%20on-x-03A9F4?logo=x)](https://x.com/intent/post?url=https%3A%2F%2Fgithub.com%2Fdryrust%2Fjq.rs&text=jq.rs)
[![Share on Reddit](https://img.shields.io/badge/share%20on-reddit-red?logo=reddit)](https://reddit.com/submit?url=https%3A%2F%2Fgithub.com%2Fdryrust%2Fjq.rs&title=jq.rs)
[![Share on Hacker News](https://img.shields.io/badge/share%20on-hn-orange?logo=ycombinator)](https://news.ycombinator.com/submitlink?u=https%3A%2F%2Fgithub.com%2Fdryrust%2Fjq.rs&t=jq.rs)
[![Share on Facebook](https://img.shields.io/badge/share%20on-fb-1976D2?logo=facebook)](https://www.facebook.com/sharer/sharer.php?u=https%3A%2F%2Fgithub.com%2Fdryrust%2Fjq.rs)
[![Share on LinkedIn](https://img.shields.io/badge/share%20on-linkedin-3949AB?logo=linkedin)](https://www.linkedin.com/sharing/share-offsite/?url=https%3A%2F%2Fgithub.com%2Fdryrust%2Fjq.rs)

[API documentation]: https://docs.rs/jq
[Crates.io]: https://crates.io/crates/jq
[Readmer]: https://github.com/artob/readmer
[Rust]: https://rust-lang.org
[jaq]: https://github.com/01mf02/jaq
[jq]: https://jqlang.org
[`log`]: https://docs.rs/log
[`JsonFilter`]: https://docs.rs/jq/latest/jq/struct.JsonFilter.html
[`JsonFilterError`]: https://docs.rs/jq/latest/jq/enum.JsonFilterError.html
[`serde_json::Value`]: https://docs.rs/serde_json/latest/serde_json/enum.Value.html
