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
        from_label: &str,
        to_label: &str,
        diff: &FileDiff,
    ) -> AppResult<DiffExplanation>;
}

pub use codex::CodexProvider;
