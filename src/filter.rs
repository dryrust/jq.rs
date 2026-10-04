// This is free and unencumbered software released into the public domain.

use crate::JsonFilterError;
#[cfg(feature = "jaq")]
use crate::jaq::Program;
#[cfg(all(feature = "jq", not(feature = "jaq")))]
use crate::jq::Program;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::String,
    vec::Vec,
};
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
    bindings: Vec<(String, Value)>,
}

impl FromStr for JsonFilter {
    type Err = JsonFilterError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::with_bindings(input, core::iter::empty::<(&str, Value)>())
    }
}

impl JsonFilter {
    /// Compiles a program with named JSON values available as `$name` variables.
    ///
    /// Names omit `$` and must match `[A-Za-z_][A-Za-z0-9_]*`. Duplicate names
    /// and the reserved names `ARGS`, `ENV`, and `__loc__` return a binding error.
    /// Values are owned by the filter and reused for each call, without
    /// interpolation into the program source. References to undeclared user
    /// variables are program compilation errors.
    ///
    /// ```
    /// use jq::JsonFilter;
    /// use serde_json::json;
    ///
    /// let filter = JsonFilter::with_bindings(
    ///     ".[] | select(.score >= $minimum)", [("minimum", json!(10))],
    /// )?;
    /// assert_eq!(filter.filter_json_all(json!([{"score": 5}, {"score": 12}]))?,
    ///            [json!({"score": 12})]);
    /// # Ok::<(), jq::JsonFilterError>(())
    /// ```
    pub fn with_bindings<N: AsRef<str>>(
        source: &str,
        bindings: impl IntoIterator<Item = (N, Value)>,
    ) -> Result<Self, JsonFilterError> {
        let bindings: Vec<_> = bindings
            .into_iter()
            .map(|(name, value)| (String::from(name.as_ref()), value))
            .collect();
        validate_bindings(&bindings)?;
        let program = Program::compile(source, &bindings)?;
        Ok(Self { program, bindings })
    }

    /// Replaces all bound values without recompiling or revalidating the source.
    ///
    /// Supply exactly the names passed to [`Self::with_bindings`], in any order.
    /// Missing, extra, invalid, or duplicate names return a binding error and
    /// leave all previous values intact. Clones keep their own binding values.
    /// This method does not change the subprocess backend's per-call compilation.
    ///
    /// ```
    /// use jq::JsonFilter;
    /// use serde_json::json;
    ///
    /// let mut filter = JsonFilter::with_bindings(". + $offset", [("offset", json!(1))])?;
    /// assert_eq!(filter.filter_json(json!(2))?, json!(3));
    /// filter.set_bindings([("offset", json!(10))])?;
    /// assert_eq!(filter.filter_json(json!(2))?, json!(12));
    /// # Ok::<(), jq::JsonFilterError>(())
    /// ```
    pub fn set_bindings<N: AsRef<str>>(
        &mut self,
        bindings: impl IntoIterator<Item = (N, Value)>,
    ) -> Result<(), JsonFilterError> {
        let bindings: Vec<_> = bindings
            .into_iter()
            .map(|(name, value)| (String::from(name.as_ref()), value))
            .collect();
        validate_bindings(&bindings)?;
        if bindings.len() != self.bindings.len() {
            return Err(JsonFilterError::Bindings(format!(
                "expected {} bindings, received {}",
                self.bindings.len(),
                bindings.len()
            )));
        }
        let mut values: BTreeMap<_, _> = bindings.into_iter().collect();
        for (name, _) in &self.bindings {
            if !values.contains_key(name) {
                return Err(JsonFilterError::Bindings(format!(
                    "missing variable: {name}"
                )));
            }
        }
        // Preserve the declaration order used by the compiled program. Validate
        // the whole name set before changing any values, so failure is atomic.
        for (name, value) in &mut self.bindings {
            *value = values.remove(name).expect("validated binding name");
        }
        Ok(())
    }

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
        self.program.visit(input, &self.bindings, visitor)
    }
}

fn validate_bindings(bindings: &[(String, Value)]) -> Result<(), JsonFilterError> {
    let mut seen = BTreeSet::new();
    for (name, _) in bindings {
        let mut bytes = name.bytes();
        let valid = bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !valid || matches!(name.as_str(), "ARGS" | "ENV" | "__loc__") {
            return Err(JsonFilterError::Bindings(format!(
                "invalid variable name: {name:?}"
            )));
        }
        if !seen.insert(name) {
            return Err(JsonFilterError::Bindings(format!(
                "duplicate variable name: {name}"
            )));
        }
    }
    Ok(())
}
