/// 压缩策略
pub enum CompactionStrategy {
    /// 保留最近 N 条，截断更早的（MVP 默认）
    KeepRecent,
    /// LLM 生成摘要替代旧事件（未来）
    LlmSummary,
}

/// 压缩结果
pub enum CompactionOutcome {
    /// 不需要压缩
    Skipped { reason: String },
    /// 已执行压缩
    Compacted {
        range_start: u64,
        range_end: u64,
        strategy: CompactionStrategy,
    },
}

/// 压缩策略配置
pub struct CompactionPolicy {
    /// 触发压缩的事件数阈值
    pub event_count_threshold: u64,
    /// 压缩后保留的最近事件数
    pub keep_recent_events: u64,
    /// 使用的压缩策略
    pub strategy: CompactionStrategy,
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            event_count_threshold: 100,
            keep_recent_events: 50,
            strategy: CompactionStrategy::KeepRecent,
        }
    }
}
