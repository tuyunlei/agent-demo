use std::collections::HashMap;

use super::*;

struct InMemoryAuthMock {
    users: HashMap<String, (String, String)>,
}

impl InMemoryAuthMock {
    fn create_user(&mut self, email: &str, password: &str, display_name: &str) {
        let password_hash = hash_password(password).expect("hash should succeed");
        self.users
            .insert(email.to_string(), (display_name.to_string(), password_hash));
    }

    fn authenticate(&self, email: &str, password: &str) -> Result<AuthResult, AuthError> {
        let (display_name, password_hash) =
            self.users.get(email).ok_or(AuthError::InvalidCredentials)?;

        if !verify_password(password, password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        Ok(AuthResult {
            user_id: "mock-user-id".to_string(),
            display_name: display_name.to_string(),
        })
    }
}

#[test]
fn create_user_stores_bcrypt_hash_instead_of_plaintext() {
    let mut store = InMemoryAuthMock {
        users: HashMap::new(),
    };

    store.create_user("alice@example.com", "password123", "Alice");
    let (_, password_hash) = store.users.get("alice@example.com").expect("user exists");

    assert_ne!(password_hash, "password123");
    assert!(verify_password("password123", password_hash).expect("verify ok"));
}

#[test]
fn authenticate_success_with_correct_password() {
    let mut store = InMemoryAuthMock {
        users: HashMap::new(),
    };
    store.create_user("alice@example.com", "password123", "Alice");

    let result = store.authenticate("alice@example.com", "password123");

    assert_eq!(
        result,
        Ok(AuthResult {
            user_id: "mock-user-id".to_string(),
            display_name: "Alice".to_string(),
        })
    );
}

#[test]
fn authenticate_fails_with_wrong_password() {
    let mut store = InMemoryAuthMock {
        users: HashMap::new(),
    };
    store.create_user("alice@example.com", "password123", "Alice");

    let result = store.authenticate("alice@example.com", "wrong-password");

    assert_eq!(result, Err(AuthError::InvalidCredentials));
}
