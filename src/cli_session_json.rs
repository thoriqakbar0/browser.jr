use std::io::{BufRead, Write};

use crate::cli::{ExitStatus, combine_status};
use crate::cli_output::{write_session_command_json, write_session_lifecycle_json};
use crate::cli_session::{CliSession, SessionStep};

pub(crate) fn run_json_session(
    input: &mut impl BufRead,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitStatus {
    if write_session_lifecycle_json(output, "ready").is_err() || output.flush().is_err() {
        return ExitStatus::Unavailable;
    }

    let mut session = CliSession::new();
    let commands_status = run_json_commands(&mut session, input, output, errors);
    let close_status = if write_session_lifecycle_json(output, "closed").is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::Unavailable
    };
    let status = combine_status(commands_status, close_status);
    if flush_streams(output, errors).is_err() {
        ExitStatus::Unavailable
    } else {
        status
    }
}

fn run_json_commands(
    session: &mut CliSession,
    input: &mut impl BufRead,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> ExitStatus {
    let mut status = ExitStatus::Success;
    let mut sequence = 1_u64;
    for line in input.lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => return write_input_error(output, sequence, status, error),
        };
        let result = run_json_command(session, &line, output, sequence);
        status = combine_status(status, result.status);
        if flush_streams(output, errors).is_err() {
            return ExitStatus::Unavailable;
        }
        if result.should_exit {
            break;
        }
        sequence = sequence
            .checked_add(1)
            .expect("session command sequence exhausted");
    }
    status
}

fn run_json_command(
    session: &mut CliSession,
    line: &str,
    output: &mut impl Write,
    sequence: u64,
) -> JsonCommandResult {
    let command = line.strip_suffix('\r').unwrap_or(line);
    let mut command_output = Vec::new();
    let mut command_errors = Vec::new();
    let step = session.run_line(command, &mut command_output, &mut command_errors);
    let result = JsonCommandResult::from_step(step);
    let command_output = session_buffer_text(command_output);
    let command_errors = session_buffer_text(command_errors);
    let command_error = result.error(&command_errors);
    if write_session_command_json(output, sequence, &command_output, command_error).is_err() {
        return JsonCommandResult::unavailable();
    }
    result
}

fn write_input_error(
    output: &mut impl Write,
    sequence: u64,
    current_status: ExitStatus,
    error: std::io::Error,
) -> ExitStatus {
    let message = format!("browser.jr: session input failed: {error}");
    if write_session_command_json(output, sequence, "", Some(&message)).is_err() {
        return ExitStatus::Unavailable;
    }
    combine_status(current_status, ExitStatus::Unavailable)
}

fn session_buffer_text(buffer: Vec<u8>) -> String {
    let text = String::from_utf8(buffer).expect("session output is valid UTF-8");
    text.strip_suffix('\n').unwrap_or(&text).to_owned()
}

fn flush_streams(output: &mut impl Write, errors: &mut impl Write) -> std::io::Result<()> {
    output.flush()?;
    errors.flush()
}

struct JsonCommandResult {
    status: ExitStatus,
    should_exit: bool,
}

impl JsonCommandResult {
    fn from_step(step: SessionStep) -> Self {
        match step {
            SessionStep::Continue(status) => Self {
                status,
                should_exit: false,
            },
            SessionStep::Exit => Self {
                status: ExitStatus::Success,
                should_exit: true,
            },
        }
    }

    fn unavailable() -> Self {
        Self {
            status: ExitStatus::Unavailable,
            should_exit: false,
        }
    }

    fn error<'a>(&self, rendered: &'a str) -> Option<&'a str> {
        if self.status == ExitStatus::Success {
            None
        } else if rendered.is_empty() {
            Some("browser.jr: session command failed")
        } else {
            Some(rendered)
        }
    }
}
