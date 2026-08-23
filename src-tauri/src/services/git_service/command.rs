use crate::error::{AppError, AppResult};
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, Output},
    time::Instant,
};

const SLOW_GIT_COMMAND_MS: u128 = 250;
const GIT_TIMING_ENV: &str = "UNDIFFSTAND_GIT_TIMING";

pub(super) fn git_output<I, S>(repo: &Path, args: I) -> AppResult<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<OsString> = args
        .into_iter()
        .map(|argument| argument.as_ref().to_owned())
        .collect();
    let operation = args
        .first()
        .and_then(|argument| argument.to_str())
        .unwrap_or("unknown");
    let started_at = Instant::now();
    let result = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(&args)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::new(
                    "GIT_NOT_FOUND",
                    "Git is not installed or is not available on PATH.",
                )
            } else {
                AppError::unknown(error)
            }
        });
    let elapsed_ms = started_at.elapsed().as_millis();
    if elapsed_ms >= SLOW_GIT_COMMAND_MS || std::env::var(GIT_TIMING_ENV).as_deref() == Ok("1") {
        let status = result
            .as_ref()
            .map(|output| output.status.code().unwrap_or(-1).to_string())
            .unwrap_or_else(|_| "spawn-error".to_owned());
        eprintln!(
            "[undiffstand][git-timing] command={operation} elapsed_ms={elapsed_ms} status={status}"
        );
    }
    result
}

pub(super) fn successful(
    output: Output,
    code: &'static str,
    message: &'static str,
) -> AppResult<Vec<u8>> {
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(AppError::new(code, message).with_detail(detail))
    }
}

pub(super) fn output_text(
    output: Output,
    code: &'static str,
    message: &'static str,
) -> AppResult<String> {
    let bytes = successful(output, code, message)?;
    String::from_utf8(bytes).map_err(AppError::unknown)
}
