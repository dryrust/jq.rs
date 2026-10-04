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

mod diagnostic;

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
        // Match upstream jq: input exhaustion is an error, while inputs simply
        // ends. jaq's default `def input: first(inputs)` silently yields nothing.
        let defs = jaq_std::defs()
            .filter(|definition| definition.name != "input")
            .chain(jaq_json::defs());
        let funs = jaq_std::funs()
            .chain(jaq_json::funs())
            .chain(core::iter::once((
                "input",
                jaq_std::v(0),
                Native::new(|_, cv| {
                    let value =
                        cv.0.inputs()
                            .next()
                            .unwrap_or_else(|| Err("auxiliary input exhausted".into()));
                    jaq_core::box_iter::box_once(
                        value.map_err(|message| jaq_json::Error::str(message).into()),
                    )
                }),
            )))
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

        let modules = loader
            .load(&arena, program)
            .map_err(|errors| JsonFilterError::Compile(diagnostic::load_errors(errors)))?;
        // Data imports introduce runtime variables too; accepting them without
        // loading their values would leave the execution context incomplete.
        jaq_core::load::import(&modules, |_| Err("data imports are not supported".into()))
            .map_err(|errors| JsonFilterError::Compile(diagnostic::load_errors(errors)))?;

        let names: Vec<_> = bindings
            .iter()
            .map(|(name, _)| format!("${name}"))
            .collect();
        let filter = jaq_core::Compiler::default()
            .with_global_vars(names.iter().map(String::as_str))
            .with_funs(funs)
            .compile(modules)
            .map_err(|errors| JsonFilterError::Compile(diagnostic::compile_errors(errors)))?;

        Ok(Self { filter })
    }
}

impl Program {
    pub(crate) fn visit<B>(
        &self,
        input: Value,
        bindings: &[(String, Value)],
        visitor: impl FnMut(Value) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B>, JsonFilterError> {
        self.visit_with_inputs(input, bindings, core::iter::empty(), visitor)
    }

    pub(crate) fn visit_with_inputs<B>(
        &self,
        input: Value,
        bindings: &[(String, Value)],
        auxiliary: impl Iterator<Item = Result<Value, String>>,
        mut visitor: impl FnMut(Value) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B>, JsonFilterError> {
        let inputs = RcIter::new(auxiliary.map(|value| value.map(Val::from)));
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use serde_json::json;

    #[test]
    fn auxiliary_values_are_consumed_in_order() -> Result<(), JsonFilterError> {
        let program = Program::compile("[., input, inputs]", &[])?;
        let auxiliary = [Ok(json!(1)), Ok(json!(2))].into_iter();
        assert_eq!(
            program.visit_with_inputs(json!(0), &[], auxiliary, ControlFlow::Break)?,
            ControlFlow::Break(json!([0, 1, 2]))
        );
        Ok(())
    }

    #[test]
    fn auxiliary_errors_are_catchable_and_exhaustion_is_explicit() -> Result<(), JsonFilterError> {
        let program = Program::compile("try input catch .", &[])?;
        assert_eq!(
            program.visit_with_inputs(
                json!(null),
                &[],
                [Err(String::from("read failed"))].into_iter(),
                ControlFlow::Break
            )?,
            ControlFlow::Break(json!("read failed"))
        );
        let program = Program::compile("input", &[])?;
        assert!(matches!(
            program.visit(json!(null), &[], ControlFlow::Break),
            Err(JsonFilterError::Execute(_))
        ));
        let program = Program::compile("inputs", &[])?;
        let mut seen = vec![];
        let status = program.visit_with_inputs(json!(null), &[], core::iter::empty(), |value| {
            seen.push(value);
            ControlFlow::<()>::Continue(())
        })?;
        assert_eq!(status, ControlFlow::Continue(()));
        assert!(seen.is_empty());
        Ok(())
    }
}
