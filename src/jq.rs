// This is free and unencumbered software released into the public domain.

use crate::{CompilationDiagnostic, CompilationPhase, JsonFilterError};
use alloc::{format, string::String, vec, vec::Vec};
use core::{
    ops::ControlFlow,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::{Mutex, MutexGuard, PoisonError, mpsc},
};

#[derive(Clone)]
pub(crate) struct Program {
    source: String,
    timeout: Option<Duration>,
}

impl Default for Program {
    fn default() -> Self {
        Self {
            source: ".".into(),
            timeout: None,
        }
    }
}

impl Program {
    pub(crate) fn compile(
        source: &str,
        bindings: &[(String, Value)],
    ) -> Result<Self, JsonFilterError> {
        // Names are validated by the common API. Values travel as JSON on stdin,
        // avoiding source interpolation and command-line argument size limits.
        let source = if bindings.is_empty() {
            format!("{DIRECTIVE_GUARD}{source}")
        } else {
            let mut prefix = String::new();
            for (index, (name, _)) in bindings.iter().enumerate() {
                prefix.push_str(&format!(".bindings[{index}] as ${name} | "));
            }
            format!("{prefix}.input |\n{source}")
        };
        // jq compiles before reading stdin. EOF validates the actual program
        // without evaluation or a wrapper that can change its syntax.
        let output = jq_command()
            .and_then(|mut command| {
                command
                    .args(["-c", "-M", "--"])
                    .arg(&source)
                    .stdin(Stdio::null())
                    .output()
            })
            .map_err(|error| {
                JsonFilterError::Compile(vec![CompilationDiagnostic {
                    phase: CompilationPhase::Load,
                    message: format!("cannot run jq: {error}"),
                    span: None,
                }])
            })?;
        if !output.status.success() {
            return Err(JsonFilterError::Compile(compile_diagnostics(&output)));
        }
        Ok(Self {
            source,
            timeout: None,
        })
    }

    pub(crate) fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
}

// Host variables often hold secrets, and jq exposes them through `env` and
// `$ENV`. Clearing them also unsets `HOME`, so `~/.jq` is not loaded.
fn jq_command() -> Result<Command, std::io::Error> {
    let mut command = Command::new(jq_executable()?);
    if cfg!(not(feature = "env")) {
        command.env_clear();
    }
    Ok(command)
}

// Search this process's `PATH` explicitly: without `PATH` in the child's
// environment, Unix `execvp` would fall back to a default search path.
fn jq_executable() -> Result<std::path::PathBuf, std::io::Error> {
    let name = format!("jq{}", std::env::consts::EXE_SUFFIX);
    std::env::var_os("PATH")
        .iter()
        .flat_map(std::env::split_paths)
        .map(|dir| dir.join(&name))
        .find(|path| path.is_file())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "jq not found in PATH"))
}

// jq accepts module directives only before any definition, so a leading one
// rejects `import`, `include`, and `module`. Imports would otherwise read jq
// modules and JSON data files relative to the working directory. The binding
// prefix and auxiliary prelude reject directives as well.
#[cfg(not(feature = "import"))]
const DIRECTIVE_GUARD: &str = "def __jqrs_guard: .; ";
#[cfg(feature = "import")]
const DIRECTIVE_GUARD: &str = "";

fn compile_diagnostics(output: &std::process::Output) -> Vec<CompilationDiagnostic> {
    let phase = if output.status.code() == Some(3) {
        CompilationPhase::Compile
    } else {
        CompilationPhase::Load
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut diagnostics: Vec<_> = stderr
        .lines()
        .filter_map(|line| {
            let message = line.strip_prefix("jq: error: ")?;
            // Source excerpts and wrapper-relative line numbers vary with the jq
            // version. Keep its explanatory text, without claiming a source span.
            let message = message.split(" at <top-level>").next()?.trim();
            Some(CompilationDiagnostic {
                phase,
                message: message.into(),
                span: None,
            })
        })
        .collect();
    if diagnostics.is_empty() {
        diagnostics.push(CompilationDiagnostic {
            phase,
            message: format!("jq compiler failed: {}", output.status),
            span: None,
        });
    }
    diagnostics
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
        auxiliary: Option<impl Iterator<Item = Result<Value, String>> + Send>,
        mut visitor: impl FnMut(Value) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B>, JsonFilterError> {
        let input = if bindings.is_empty() {
            input
        } else {
            serde_json::json!({
                "input": input,
                "bindings": bindings.iter().map(|(_, value)| value).collect::<alloc::vec::Vec<_>>()
            })
        };
        let mut command = jq_command().map_err(io_error)?;
        command.args(["-c", "-M", "--unbuffered"]);
        if auxiliary.is_some() {
            // Evaluate the main input exactly once. Tagged records carry either
            // an auxiliary JSON value or a catchable input error, without
            // confusing user arrays/objects with transport metadata.
            command.args(["-n", "--"]).arg(format!(
                "{AUXILIARY_PRELUDE}\n__jqrs_raw_input |\n{}",
                self.source
            ));
        } else {
            command.arg("--").arg(&self.source);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(io_error)?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let mut stderr = child.stderr.take().expect("piped stderr");
        // Shared with the watchdog, which must be able to kill a running jq.
        let child = Mutex::new(child);
        let timed_out = AtomicBool::new(false);

        std::thread::scope(|scope| {
            // Drop the guard before scoped threads are joined, including on
            // visitor panics: killing jq releases the pipe worker threads.
            let mut guard = ChildGuard {
                child: &child,
                reaped: false,
            };
            // Dropping the sender on any return also stops the watchdog.
            let (_stop, stopped) = mpsc::channel::<()>();
            if let Some(timeout) = self.timeout {
                let (child, timed_out) = (&child, &timed_out);
                scope.spawn(move || {
                    if stopped.recv_timeout(timeout) == Err(mpsc::RecvTimeoutError::Timeout) {
                        let mut child = lock(child);
                        // Only a still-running jq counts as having timed out.
                        if let Ok(None) = child.try_wait() {
                            timed_out.store(true, Ordering::SeqCst);
                            let _ = child.kill();
                        }
                    }
                });
            }
            // Killing jq can truncate its output, so report the cause instead.
            let timeout_or = |error| {
                if timed_out.load(Ordering::SeqCst) {
                    JsonFilterError::Timeout
                } else {
                    error
                }
            };
            // Write and drain concurrently to avoid pipe-capacity deadlocks.
            let writer = scope.spawn(move || {
                write_record(&mut stdin, &input)?;
                for item in auxiliary.into_iter().flatten() {
                    let record = match item {
                        Ok(value) => serde_json::json!([true, value]),
                        Err(message) => serde_json::json!([false, message]),
                    };
                    write_record(&mut stdin, &record)?;
                }
                Ok::<_, std::io::Error>(())
            });
            let errors = scope.spawn(move || {
                let mut bytes = vec![];
                stderr.read_to_end(&mut bytes).map_err(io_error)?;
                Ok::<_, JsonFilterError>(bytes)
            });

            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                if reader
                    .read_line(&mut line)
                    .map_err(|error| timeout_or(io_error(error)))?
                    == 0
                {
                    break;
                }
                let value = serde_json::from_str(&line)
                    .map_err(|error| timeout_or(JsonFilterError::Output(error)))?;
                if let ControlFlow::Break(value) = visitor(value) {
                    // Drop kills and reaps the producer before joining workers.
                    return Ok(ControlFlow::Break(value));
                }
            }
            let status = lock(&child).wait().map_err(io_error)?;
            guard.reaped = true;
            let stderr = errors.join().map_err(|_| worker_error())??;
            let written = writer.join().map_err(|_| worker_error())?;
            if !status.success() {
                return Err(timeout_or(JsonFilterError::Execute(format!(
                    "jq {status}: {}",
                    String::from_utf8_lossy(&stderr).trim()
                ))));
            }
            // A successful filter need not consume the entire auxiliary stream.
            if let Err(error) = written
                && error.kind() != std::io::ErrorKind::BrokenPipe
            {
                return Err(io_error(error));
            }
            Ok(ControlFlow::Continue(()))
        })
    }
}

const AUXILIARY_PRELUDE: &str = r#"
def __jqrs_raw_input: input;
def __jqrs_raw_inputs: inputs;
def __jqrs_decode: if .[0] then .[1] else error(.[1]) end;
def input: __jqrs_raw_input | __jqrs_decode;
def inputs: __jqrs_raw_inputs | __jqrs_decode;
"#;

fn write_record(writer: &mut impl Write, value: &Value) -> Result<(), std::io::Error> {
    serde_json::to_writer(&mut *writer, value).map_err(|error| {
        std::io::Error::new(
            error.io_error_kind().unwrap_or(std::io::ErrorKind::Other),
            error,
        )
    })?;
    writer.write_all(b"\n")
}

struct ChildGuard<'a> {
    child: &'a Mutex<Child>,
    reaped: bool,
}

impl Drop for ChildGuard<'_> {
    fn drop(&mut self) {
        if !self.reaped {
            let mut child = lock(self.child);
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// A panicking visitor never holds the lock, but stay robust regardless.
fn lock(child: &Mutex<Child>) -> MutexGuard<'_, Child> {
    child.lock().unwrap_or_else(PoisonError::into_inner)
}

fn io_error(error: impl core::fmt::Display) -> JsonFilterError {
    JsonFilterError::Execute(format!("jq I/O error: {error}"))
}

fn worker_error() -> JsonFilterError {
    JsonFilterError::Execute("jq pipe worker panicked".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn auxiliary_records_preserve_values_and_errors() -> Result<(), JsonFilterError> {
        let program = Program::compile("[., input, (try input catch .), inputs]", &[])?;
        let auxiliary = [
            Ok(json!([false, "literal"])),
            Err(String::from("read failed")),
            Ok(json!({"value": true})),
        ]
        .into_iter();
        assert_eq!(
            program.visit_with_inputs(json!(0), &[], Some(auxiliary), ControlFlow::Break)?,
            ControlFlow::Break(json!([0, [false, "literal"], "read failed", {"value": true}]))
        );
        Ok(())
    }

    #[test]
    fn unused_auxiliary_values_do_not_become_main_inputs() -> Result<(), JsonFilterError> {
        let program = Program::compile(".", &[])?;
        let mut values = vec![];
        let status = program.visit_with_inputs(
            json!(0),
            &[],
            Some(core::iter::repeat_with(|| Ok(json!("x".repeat(64 * 1024))))),
            |value| {
                values.push(value);
                ControlFlow::<()>::Continue(())
            },
        )?;
        assert_eq!(status, ControlFlow::Continue(()));
        assert_eq!(values, [json!(0)]);
        Ok(())
    }
}
