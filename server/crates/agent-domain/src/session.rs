#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionStatus {
    Active,
    Compacting,
    Archived,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub tenant_id: String,
    pub status: SessionStatus,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_active_at: i64,
    pub event_count: u64,
    pub token_estimate: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_construct_and_serialize() {
        let session = Session {
            id: "s-1".to_string(),
            user_id: "u-1".to_string(),
            tenant_id: "t-1".to_string(),
            status: SessionStatus::Active,
            title: Some("demo".to_string()),
            created_at: 1,
            updated_at: 2,
            last_active_at: 3,
            event_count: 10,
            token_estimate: Some(128),
        };

        let json = serde_json::to_string(&session).expect("serialize session");
        let decoded: Session = serde_json::from_str(&json).expect("deserialize session");

        assert_eq!(decoded, session);
    }
}
