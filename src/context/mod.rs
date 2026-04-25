mod adaptive;
mod budget;
pub mod cache;
pub mod compaction;
mod files;
pub mod handoff;
pub(crate) mod injection;
mod loader;
pub mod progressive;
mod summarize;

pub(crate) use adaptive::{adapt_config, record_context_feedback};
pub use budget::{BudgetedContext, BudgetedSource, ContextBudgeter, ContextSourceInput};
pub use cache::{
    ContextCache, ContextCacheEntry, load_context_cache, render_context_cache, write_context_cache,
};
pub use compaction::{
    AUTO_COMPACT_THRESHOLD, CompactionRequest, CompactionTrigger, compact_if_needed,
    consume_compact, is_pending as compaction_pending, request_compact,
};
pub use files::TrackedContextReader;
pub(crate) use loader::{ContextLoadRequest, LocalContextLoader};
