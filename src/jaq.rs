// This is free and unencumbered software released into the public domain.

use crate::JsonFilterError;
use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
use core::ops::ControlFlow;
use jaq_core::{
    Ctx, DataT, Filter, Lut, Native, Vars,
    data::HasLut,
    load::{Arena, File, Loader},
    native::v,
};
use jaq_json::{Num, Val};
use jaq_std::input::{HasInputs, Inputs, RcIter};
use serde_json::Value;

mod diagnostic;

#[derive(Clone, Default)]
pub(crate) struct Program {
    filter: Rc<Filter<Data>>,
}

struct Data;

impl DataT for Data {
    type V<'a> = Val;
    type Data<'a> = Runtime<'a>;
}

#[derive(Clone, Copy)]
struct Runtime<'a> {
    lut: &'a Lut<Data>,
    inputs: Inputs<'a, Val>,
}

impl<'a> HasLut<'a, Data> for Runtime<'a> {
    fn lut(&self) -> &'a Lut<Data> {
        self.lut
    }
}

impl<'a> HasInputs<'a, Val> for Runtime<'a> {
    fn inputs(&self) -> Inputs<'a, Val> {
        self.inputs
    }
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
        let defs = jaq_core::defs()
            .chain(jaq_std::defs())
            .chain(jaq_json::defs())
            .filter(|definition| !matches!(definition.name, "halt" | "halt_error"));
        #[cfg(feature = "std")]
        let std_funs = jaq_std::funs::<Data>();
        #[cfg(not(feature = "std"))]
        let std_funs = jaq_std::base_funs::<Data>();
        let halts = [
            ("halt", 0),
            ("halt", 1),
            ("halt_error", 0),
            ("halt_error", 1),
        ]
        .into_iter()
        .map(|(name, arity)| {
            (
                name,
                v(arity),
                Native::<Data>::new(|_| {
                    jaq_core::box_iter::box_once(Err(jaq_json::Error::str(
                        "process termination is disabled",
                    )
                    .into()))
                }),
            )
        });
        let funs = jaq_core::funs::<Data>()
            .chain(std_funs)
            .chain(jaq_json::funs::<Data>())
            .filter(|(name, _, _)| !matches!(*name, "halt" | "halt_error"))
            .chain(halts)
            .chain(
                jaq_std::input::funs::<Data>()
                    .into_vec()
                    .into_iter()
                    .map(|(name, args, run)| (name, args, Native::<Data>::new(run))),
            )
            .map(|(name, args, implementation)| {
                let implementation = match name {
                    "length" => Native::<Data>::new(|cv| {
                        jaq_core::box_iter::box_once(length(cv.1).map_err(Into::into))
                    }),
                    "input" => Native::<Data>::new(|cv| {
                        let value =
                            cv.0.data()
                                .inputs()
                                .next()
                                .unwrap_or_else(|| Err("auxiliary input exhausted".into()));
                        jaq_core::box_iter::box_once(
                            value.map_err(|message| jaq_json::Error::str(message).into()),
                        )
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

        Ok(Self {
            filter: Rc::new(filter),
        })
    }
}

impl Program {
    pub(crate) fn visit<B>(
        &self,
        input: Value,
        bindings: &[(String, Value)],
        visitor: impl FnMut(Value) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B>, JsonFilterError> {
        self.visit_with_inputs(input, bindings, None::<core::iter::Empty<_>>, visitor)
    }

    pub(crate) fn visit_with_inputs<B>(
        &self,
        input: Value,
        bindings: &[(String, Value)],
        auxiliary: Option<impl Iterator<Item = Result<Value, String>>>,
        mut visitor: impl FnMut(Value) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B>, JsonFilterError> {
        let inputs = RcIter::new(
            auxiliary
                .into_iter()
                .flatten()
                .map(|value| value.map(from_json)),
        );
        let values = bindings.iter().map(|(_, value)| from_json(value.clone()));
        let context = Ctx::<Data>::new(
            Runtime {
                lut: &self.filter.lut,
                inputs: &inputs,
            },
            Vars::new(values),
        );
        for output in self
            .filter
            .id
            .run((context, from_json(input)))
            .map(jaq_core::unwrap_valr)
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

// jaq-json 2.0.3 uses isize::abs for numeric length, which overflows at MIN.
fn length(value: Val) -> jaq_json::ValR {
    Ok(match value {
        Val::Null => Val::from(0usize),
        Val::Bool(_) => return Err(jaq_json::Error::str("boolean has no length")),
        Val::Num(Num::Int(value)) => Val::Num(Num::from_integral(value.unsigned_abs())),
        Val::Num(Num::Float(value)) => Val::from(value.abs()),
        Val::Num(Num::Dec(value)) => return length(Val::Num(Num::from_dec_str(&value))),
        value @ Val::Num(Num::BigInt(_)) => {
            if value < Val::from(0isize) {
                return -value;
            }
            value
        }
        Val::BStr(value) => Val::from(value.len()),
        Val::TStr(value) => Val::from(String::from_utf8_lossy(&value).chars().count()),
        Val::Arr(value) => Val::from(value.len()),
        Val::Obj(value) => Val::from(value.len()),
    })
}

fn from_json(value: Value) -> Val {
    match value {
        Value::Null => Val::Null,
        Value::Bool(value) => Val::Bool(value),
        Value::Number(value) => {
            let text = value.to_string();
            Val::Num(Num::from_str_radix(&text, 10).unwrap_or_else(|| Num::Dec(text.into())))
        }
        Value::String(value) => Val::from(value),
        Value::Array(values) => values.into_iter().map(from_json).collect(),
        Value::Object(values) => Val::obj(
            values
                .into_iter()
                .map(|(key, value)| (Val::from(key), from_json(value)))
                .collect(),
        ),
    }
}

// Check every nested number rather than using jaq-json's panicking conversion.
fn to_json(value: &Val) -> Result<Value, JsonFilterError> {
    Ok(match value {
        Val::Null => Value::Null,
        Val::Bool(value) => Value::Bool(*value),
        Val::Num(Num::Int(value)) => Value::Number((*value).into()),
        Val::Num(Num::Float(value)) => {
            serde_json::Number::from_f64(*value).map_or(Value::Null, Value::Number)
        }
        Val::Num(value) => {
            Value::Number(value.to_string().parse().map_err(JsonFilterError::Output)?)
        }
        Val::TStr(value) => Value::String(
            core::str::from_utf8(value)
                .map_err(|error| JsonFilterError::OutputValue(error.to_string()))?
                .into(),
        ),
        Val::BStr(_) => {
            return Err(JsonFilterError::OutputValue(
                "binary strings are not JSON strings".into(),
            ));
        }
        Val::Arr(values) => Value::Array(values.iter().map(to_json).collect::<Result<_, _>>()?),
        Val::Obj(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| {
                    let Value::String(key) = to_json(key)? else {
                        return Err(JsonFilterError::OutputValue(
                            "JSON object keys must be strings".into(),
                        ));
                    };
                    Ok((key, to_json(value)?))
                })
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
            program.visit_with_inputs(json!(0), &[], Some(auxiliary), ControlFlow::Break)?,
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
                Some([Err(String::from("read failed"))].into_iter()),
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
        let status =
            program.visit_with_inputs(json!(null), &[], Some(core::iter::empty()), |value| {
                seen.push(value);
                ControlFlow::<()>::Continue(())
            })?;
        assert_eq!(status, ControlFlow::Continue(()));
        assert!(seen.is_empty());
        Ok(())
    }
}
