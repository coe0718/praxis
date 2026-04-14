mod adaptive;
mod budget;
pub mod compaction;
mod files;
pub mod handoff;
mod loader;
mod summarize;

pub(crate) use adaptive::{adapt_config, record_context_feedback};
pub use budget::{BudgetedContext, BudgetedSource, ContextBudgeter, ContextSourceInput};
pub use compaction::{
    AUTO_COMPACT_THRESHOLD, CompactionRequest, CompactionTrigger, compact_if_needed,
    consume_compact, is_pending as compaction_pending, request_compact,
};
pub use files::TrackedContextReader;
pub(crate) use loader::{ContextLoadRequest, LocalContextLoader};
