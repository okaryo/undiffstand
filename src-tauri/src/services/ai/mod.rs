mod codex;

use crate::{
    domain::{DiffExplanation, FileDiff},
    error::AppResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn explain_file_diff(
        &self,
        base_ref: &str,
        head_sha: &str,
        diff: &FileDiff,
    ) -> AppResult<DiffExplanation>;
}

pub use codex::CodexProvider;
