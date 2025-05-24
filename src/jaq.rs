// This is free and unencumbered software released into the public domain.

use alloc::{format, string::String, vec::Vec};
use core::str::FromStr;
use jaq_core::{
    Ctx, Filter, Native, RcIter,
    load::{Arena, File, Loader},
};
use jaq_json::Val;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum JsonFilterError {
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("compilation error: {0:?}")]
    Compile(Vec<String>),

    #[error("no output")]
    NoOutput,

    #[error("execution error: {0}")]
    Execute(jaq_json::Error),
}

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
    pub fn filter_json_str(&self, input: impl AsRef<str>) -> Result<Value, JsonFilterError> {
        self.filter_json(serde_json::from_str(input.as_ref())?)
    }

    pub fn filter_json(&self, input: Value) -> Result<Value, JsonFilterError> {
        let inputs = RcIter::new(core::iter::empty());
        let mut outputs = self.filter.run((Ctx::new([], &inputs), Val::from(input)));
        Ok(outputs
            .next()
            .ok_or_else(|| JsonFilterError::NoOutput)?
            .map_err(|e| JsonFilterError::Execute(e))?
            .into())
    }
}
