mod codex;

use crate::{
    domain::{AiAnswer, CodeSelection, DiffExplanation, FileDiff},
    error::AppResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn ask_about_code(
        &self,
        selection: &CodeSelection,
        question: &str,
        surrounding_code: &str,
    ) -> AppResult<AiAnswer>;

    async fn explain_file_diff(
        &self,
        base_ref: &str,
        head_sha: &str,
        diff: &FileDiff,
    ) -> AppResult<DiffExplanation>;
}

pub use codex::CodexProvider;
