// This is free and unencumbered software released into the public domain.

use crate::JsonFilterError;
use alloc::{format, string::ToString, vec::Vec};
use core::str::FromStr;
use jaq_core::{
    Ctx, Filter, Native, RcIter,
    load::{Arena, File, Loader},
};
use jaq_json::Val;
use serde_json::Value;

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
        let funs = jaq_std::funs()
            .chain(jaq_json::funs())
            .map(|(name, args, implementation)| {
                // Process termination is inappropriate in an embedded filter.
                let implementation = match name {
                    "halt" | "halt_error" => Native::new(|_, _| {
                        jaq_core::box_iter::box_once(Err(jaq_json::Error::str(
                            "process termination is disabled",
                        )
                        .into()))
                    }),
                    _ => implementation,
                };
                (name, args, implementation)
            });

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
        let output = outputs
            .next()
            .ok_or(JsonFilterError::NoOutput)?
            .map_err(execution_error)?;
        to_json(&output)
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
            .map(|output| to_json(&output.map_err(execution_error)?))
            .collect()
    }
}

fn execution_error(error: jaq_json::Error) -> JsonFilterError {
    JsonFilterError::Execute(error.to_string())
}

// Check every nested number rather than using jaq-json's panicking conversion.
fn to_json(value: &Val) -> Result<Value, JsonFilterError> {
    Ok(match value {
        Val::Null => Value::Null,
        Val::Bool(value) => Value::Bool(*value),
        Val::Int(value) => Value::Number((*value).into()),
        Val::Float(value) => {
            serde_json::Number::from_f64(*value).map_or(Value::Null, Value::Number)
        }
        Val::Num(value) => Value::Number(value.parse().map_err(JsonFilterError::Output)?),
        Val::Str(value) => Value::String((**value).clone()),
        Val::Arr(values) => Value::Array(values.iter().map(to_json).collect::<Result<_, _>>()?),
        Val::Obj(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| Ok(((**key).clone(), to_json(value)?)))
                .collect::<Result<_, JsonFilterError>>()?,
        ),
    })
}
