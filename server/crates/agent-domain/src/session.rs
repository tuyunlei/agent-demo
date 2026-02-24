#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionStatus {
    Active,
    Compacting,
    Archived,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub session_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub status: SessionStatus,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_active_at: i64,
    pub event_count: u64,
    pub last_sequence: u64,
    pub estimated_prompt_tokens: Option<u32>,
    pub estimated_tokens_after_compaction: Option<u32>,
    pub compacted_until_sequence: Option<u64>,
    pub version: u64,
    pub archived_at: Option<i64>,
    pub last_message_at: Option<i64>,
    pub archived: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_construct_and_serialize() {
        let session = Session {
            session_id: "s-1".to_string(),
            user_id: "u-1".to_string(),
            tenant_id: "t-1".to_string(),
            agent_id: "a-1".to_string(),
            status: SessionStatus::Active,
            title: Some("demo".to_string()),
            created_at: 1,
            updated_at: 2,
            last_active_at: 3,
            event_count: 10,
            last_sequence: 10,
            estimated_prompt_tokens: Some(128),
            estimated_tokens_after_compaction: None,
            compacted_until_sequence: None,
            version: 1,
            archived_at: None,
            last_message_at: None,
            archived: false,
        };

        let json = serde_json::to_string(&session).expect("serialize session");
        let decoded: Session = serde_json::from_str(&json).expect("deserialize session");

        assert_eq!(decoded, session);
    }
}
