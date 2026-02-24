#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: i64,
    pub refresh_token_expires_at: i64,
}

#[cfg(test)]
mod tests {
    use super::TokenPair;

    #[test]
    fn token_pair_can_be_constructed_and_serialized() {
        let pair = TokenPair {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            access_token_expires_at: 1,
            refresh_token_expires_at: 2,
        };

        let json = serde_json::to_string(&pair).expect("serialize token pair");
        let decoded: TokenPair = serde_json::from_str(&json).expect("deserialize token pair");

        assert_eq!(decoded, pair);
    }
}
