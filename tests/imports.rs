// This is free and unencumbered software released into the public domain.

#![cfg(all(feature = "jq", not(feature = "jaq")))]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Value, json};

#[test]
fn module_directives_require_opt_in() -> Result<(), JsonFilterError> {
    // jq resolves data imports relative to its working directory.
    let directory = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let Ok(relative) = directory.strip_prefix(std::env::current_dir().unwrap()) else {
        eprintln!("skipped: target directory is outside the working directory");
        return Ok(());
    };
    std::fs::write(directory.join("jqrs_secret.json"), r#"{"secret":1}"#).unwrap();
    std::fs::write(directory.join("jqrs_module.jq"), "def secret: 1;").unwrap();
    let path = relative.to_str().unwrap();
    for (source, expected) in [
        (
            format!(r#"import "{path}/jqrs_secret" as $s; $s"#),
            json!([{"secret": 1}]),
        ),
        (format!(r#"include "{path}/jqrs_module"; secret"#), json!(1)),
        (String::from("module {}; ."), Value::Null),
    ] {
        let filter = source.parse::<JsonFilter>();
        if cfg!(feature = "import") {
            assert_eq!(filter?.filter_json(Value::Null)?, expected, "{source}");
        } else {
            assert!(
                matches!(filter, Err(JsonFilterError::Compile(_))),
                "{source}"
            );
        }
    }
    Ok(())
}

#[test]
fn programs_without_expressions_keep_their_behavior() -> Result<(), JsonFilterError> {
    // Upstream jq rejects these without the guard as well.
    for source in ["", "# comment", "def f: 1;"] {
        assert!(
            matches!(
                source.parse::<JsonFilter>(),
                Err(JsonFilterError::Compile(_))
            ),
            "{source:?}"
        );
    }
    let filter: JsonFilter = "$__loc__.line".parse()?;
    assert_eq!(filter.filter_json(Value::Null)?, json!(1));
    Ok(())
}
