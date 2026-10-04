// This is free and unencumbered software released into the public domain.

use crate::{JsonFilterError, jaq::Program};
use alloc::vec::Vec;
use core::{ops::ControlFlow, str::FromStr};
use serde_json::Value;

/// A compiled jq-style program that can be reused across JSON inputs.
///
/// Parse a program with [`core::str::FromStr`]. [`Default`] constructs the
/// identity filter (`.`); cloning preserves the compiled program. Each method
/// starts a fresh evaluation with the supplied input.
#[derive(Clone, Default)]
pub struct JsonFilter {
    program: Program,
}

impl FromStr for JsonFilter {
    type Err = JsonFilterError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        input.parse().map(|program| Self { program })
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
        self.filter_json_visit(input, ControlFlow::Break)?
            .break_value()
            .ok_or(JsonFilterError::NoOutput)
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
        let mut values = Vec::new();
        let _ = self.filter_json_visit(input, |value| {
            values.push(value);
            ControlFlow::<()>::Continue(())
        })?;
        Ok(values)
    }

    /// Delivers results in order without buffering them in the wrapper.
    ///
    /// Return [`ControlFlow::Break`] to stop immediately and return its value.
    /// Natural exhaustion returns [`ControlFlow::Continue`], including for an
    /// empty output stream. Evaluation or conversion errors stop iteration and
    /// are returned after any earlier values have reached the visitor. Errors
    /// after an early break are not evaluated. The filter itself may still
    /// buffer intermediate values, for example when sorting an array.
    ///
    /// ```
    /// use core::ops::ControlFlow;
    /// use jq::JsonFilter;
    /// use serde_json::json;
    ///
    /// let filter: JsonFilter = "repeat(.)".parse()?;
    /// let mut count = 0;
    /// let stopped = filter.filter_json_visit(json!(1), |_| {
    ///     count += 1;
    ///     if count == 3 { ControlFlow::Break(count) }
    ///     else { ControlFlow::Continue(()) }
    /// })?;
    /// assert_eq!(stopped, ControlFlow::Break(3));
    /// # Ok::<(), jq::JsonFilterError>(())
    /// ```
    pub fn filter_json_visit<B>(
        &self,
        input: Value,
        visitor: impl FnMut(Value) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B>, JsonFilterError> {
        self.program.visit(input, visitor)
    }
}
