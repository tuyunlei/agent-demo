pub mod compaction;
pub mod types;

pub use compaction::{CompactionError, CompactionService, NoopCompactionService};
pub use types::{CompactionOutcome, CompactionPolicy, CompactionStrategy};
