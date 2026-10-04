// This is free and unencumbered software released into the public domain.

use crate::JsonFilterError;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::ops::ControlFlow;
use jaq_core::{
    Ctx, Filter, Native, RcIter,
    load::{Arena, File, Loader},
};
use jaq_json::Val;
use serde_json::Value;

#[derive(Clone, Default)]
pub(crate) struct Program {
    filter: Filter<Native<Val>>,
}

impl Program {
    pub(crate) fn compile(
        input: &str,
        bindings: &[(String, Value)],
    ) -> Result<Self, JsonFilterError> {
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

        let names: Vec<_> = bindings
            .iter()
            .map(|(name, _)| format!("${name}"))
            .collect();
        let filter = jaq_core::Compiler::default()
            .with_global_vars(names.iter().map(String::as_str))
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

        Ok(Self { filter })
    }
}

impl Program {
    pub(crate) fn visit<B>(
        &self,
        input: Value,
        bindings: &[(String, Value)],
        mut visitor: impl FnMut(Value) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B>, JsonFilterError> {
        let inputs = RcIter::new(core::iter::empty());
        let values = bindings.iter().map(|(_, value)| Val::from(value.clone()));
        for output in self
            .filter
            .run((Ctx::new(values, &inputs), Val::from(input)))
        {
            let value = to_json(&output.map_err(execution_error)?)?;
            if let ControlFlow::Break(value) = visitor(value) {
                return Ok(ControlFlow::Break(value));
            }
        }
        Ok(ControlFlow::Continue(()))
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
