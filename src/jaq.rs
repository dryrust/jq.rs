// This is free and unencumbered software released into the public domain.

use alloc::{format, string::String, vec::Vec};
use core::str::FromStr;
use jaq_core::{
    Ctx, Filter, Native, RcIter,
    load::{Arena, File, Loader},
};
use jaq_json::Val;
use serde_json::Value;

/// A failure to parse JSON, compile a filter, or evaluate it.
#[derive(Debug, thiserror::Error)]
pub enum JsonFilterError {
    /// The input string is not exactly one valid JSON value.
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// The filter program could not be loaded or compiled.
    #[error("compilation error: {0:?}")]
    Compile(Vec<String>),

    /// A single-result method evaluated a filter that produced no values.
    #[error("no output")]
    NoOutput,

    /// Evaluation failed, for example because of an invalid operand type.
    #[error("execution error: {0}")]
    Execute(jaq_json::Error),
}

/// A compiled jq-style program that can be reused across JSON inputs.
///
/// Parse a program with [`core::str::FromStr`]. [`Default`] constructs the
/// identity filter (`.`); cloning preserves the compiled program. Each method
/// starts a fresh evaluation with the supplied input.
#[derive(Clone, Default)]
pub struct JsonFilter {
    filter: Filter<Native<Val>>,
}

impl FromStr for JsonFilter {
    type Err = JsonFilterError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let program = File {
            code: input,
            path: (),
        };
        let defs = jaq_std::defs().chain(jaq_json::defs());
        let funs = jaq_std::funs().chain(jaq_json::funs());

        let loader = Loader::new(defs);
        let arena = Arena::default();

        let modules = loader.load(&arena, program).map_err(|errors| {
            JsonFilterError::Compile(
                errors
                    .into_iter()
                    .map(|error| format!("{error:?}"))
                    .collect::<Vec<_>>(),
            )
        })?;

        let filter = jaq_core::Compiler::default()
            .with_funs(funs)
            .compile(modules)
            .map_err(|errors| {
                JsonFilterError::Compile(
                    errors
                        .into_iter()
                        .map(|error| format!("{error:?}"))
                        .collect::<Vec<_>>(),
                )
            })?;

        Ok(JsonFilter { filter })
    }
}

impl JsonFilter {
    /// Parses one JSON value and returns only its first filter result.
    ///
    /// Use [`Self::filter_json_str_all`] to retain all results and detect errors
    /// after the first result.
    pub fn filter_json_str(&self, input: impl AsRef<str>) -> Result<Value, JsonFilterError> {
        self.filter_json(serde_json::from_str(input.as_ref())?)
    }

    /// Returns only the first filter result, or [`JsonFilterError::NoOutput`].
    ///
    /// Later results and errors are not evaluated. Use [`Self::filter_json_all`]
    /// to retain all results and detect errors after the first result.
    pub fn filter_json(&self, input: Value) -> Result<Value, JsonFilterError> {
        let inputs = RcIter::new(core::iter::empty());
        let mut outputs = self.filter.run((Ctx::new([], &inputs), Val::from(input)));
        Ok(outputs
            .next()
            .ok_or_else(|| JsonFilterError::NoOutput)?
            .map_err(JsonFilterError::Execute)?
            .into())
    }

    /// Parses one JSON value and collects all filter results in order.
    ///
    /// Returns an empty vector for filters producing no output. Invalid JSON
    /// or any execution error returns an error, without partial results.
    pub fn filter_json_str_all(
        &self,
        input: impl AsRef<str>,
    ) -> Result<Vec<Value>, JsonFilterError> {
        self.filter_json_all(serde_json::from_str(input.as_ref())?)
    }

    /// Collects all filter results in order.
    ///
    /// Returns an empty vector for filters producing no output. Stops at the
    /// first execution error, returning it without partial results, even if
    /// earlier results succeeded. All results are buffered in memory.
    ///
    /// ```
    /// use jq::JsonFilter;
    /// use serde_json::json;
    ///
    /// let filter: JsonFilter = ".[]".parse()?;
    /// assert_eq!(filter.filter_json_all(json!([1, 2]))?, [json!(1), json!(2)]);
    /// # Ok::<(), jq::JsonFilterError>(())
    /// ```
    pub fn filter_json_all(&self, input: Value) -> Result<Vec<Value>, JsonFilterError> {
        let inputs = RcIter::new(core::iter::empty());
        self.filter
            .run((Ctx::new([], &inputs), Val::from(input)))
            .map(|output| output.map(Value::from).map_err(JsonFilterError::Execute))
            .collect()
    }
}
