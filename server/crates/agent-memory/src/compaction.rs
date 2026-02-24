use crate::types::{CompactionOutcome, CompactionPolicy};

/// 压缩服务 trait
///
/// 负责判断是否需要压缩、执行压缩流程。
/// 具体的事件读写通过注入的 EventStore 完成（在编排层组装）。
#[async_trait::async_trait]
pub trait CompactionService: Send + Sync {
    /// 检查并执行压缩（如果需要）
    async fn compact_if_needed(
        &self,
        session_id: &str,
    ) -> Result<CompactionOutcome, CompactionError>;

    /// 获取当前压缩策略配置
    fn policy(&self) -> &CompactionPolicy;
}

#[derive(Debug, thiserror::Error)]
pub enum CompactionError {
    #[error("compaction already in progress for session {0}")]
    AlreadyInProgress(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("internal error: {0}")]
    Internal(String),
}

/// 空实现，永远不压缩。MVP 阶段使用。
pub struct NoopCompactionService {
    policy: CompactionPolicy,
}

impl NoopCompactionService {
    pub fn new() -> Self {
        Self {
            policy: CompactionPolicy::default(),
        }
    }
}

impl Default for NoopCompactionService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CompactionService for NoopCompactionService {
    async fn compact_if_needed(
        &self,
        _session_id: &str,
    ) -> Result<CompactionOutcome, CompactionError> {
        Ok(CompactionOutcome::Skipped {
            reason: "noop compaction service".to_string(),
        })
    }

    fn policy(&self) -> &CompactionPolicy {
        &self.policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CompactionStrategy;

    #[tokio::test]
    async fn noop_compaction_always_skips() {
        let service = NoopCompactionService::new();
        let result = service.compact_if_needed("test-session").await.unwrap();
        assert!(matches!(result, CompactionOutcome::Skipped { .. }));
    }

    #[test]
    fn default_policy_values() {
        let policy = CompactionPolicy::default();
        assert_eq!(policy.event_count_threshold, 100);
        assert_eq!(policy.keep_recent_events, 50);
        assert!(matches!(policy.strategy, CompactionStrategy::KeepRecent));
    }
}
