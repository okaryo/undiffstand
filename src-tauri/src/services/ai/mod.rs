mod codex;
mod skills;

use crate::{
    domain::{DiffExplanation, FileDiff, InlineAnswer, InlineQuestion, ReviewOutputLanguage},
    error::AppResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn explain_file_change(
        &self,
        from_label: &str,
        to_label: &str,
        diff: &FileDiff,
        language: ReviewOutputLanguage,
    ) -> AppResult<DiffExplanation>;

    async fn answer_inline_question(
        &self,
        from_label: &str,
        to_label: &str,
        diff: &FileDiff,
        question: &InlineQuestion,
        selected_source: &str,
        language: ReviewOutputLanguage,
    ) -> AppResult<InlineAnswer>;
}

pub use codex::CodexProvider;
