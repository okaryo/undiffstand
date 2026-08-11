use crate::error::{AppError, AppResult};
use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, Output},
};

pub(super) fn git_output<I, S>(repo: &Path, args: I) -> AppResult<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
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
        })
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
