use agent_domain::{AuthError, AuthPort, AuthResult};
use bcrypt::{DEFAULT_COST, hash, verify};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresUserStore {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    display_name: String,
    password_hash: String,
}

impl PostgresUserStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn create_user_record(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> Result<AuthResult, AuthError> {
        let password_hash = hash_password(password)?;

        let record = sqlx::query_as::<_, UserRow>(
            "INSERT INTO users (email, password_hash, display_name)\
             VALUES ($1, $2, $3)\
             RETURNING id, display_name, password_hash",
        )
        .bind(email)
        .bind(password_hash)
        .bind(display_name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(AuthResult {
            user_id: record.id.to_string(),
            display_name: record.display_name,
        })
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<AuthResult>, AuthError> {
        let record = sqlx::query_as::<_, UserRow>(
            "SELECT id, display_name, password_hash FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(record.map(|row| AuthResult {
            user_id: row.id.to_string(),
            display_name: row.display_name,
        }))
    }

    async fn find_user_row_by_email(&self, email: &str) -> Result<Option<UserRow>, AuthError> {
        sqlx::query_as::<_, UserRow>(
            "SELECT id, display_name, password_hash FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }
}

#[async_trait::async_trait]
impl AuthPort for PostgresUserStore {
    async fn authenticate(&self, email: &str, password: &str) -> Result<AuthResult, AuthError> {
        let user = self
            .find_user_row_by_email(email)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        let password_ok = verify_password(password, &user.password_hash)?;
        if !password_ok {
            return Err(AuthError::InvalidCredentials);
        }

        Ok(AuthResult {
            user_id: user.id.to_string(),
            display_name: user.display_name,
        })
    }

    async fn create_user(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> Result<AuthResult, AuthError> {
        self.create_user_record(email, password, display_name).await
    }
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    hash(password, DEFAULT_COST).map_err(|err| AuthError::Internal(err.to_string()))
}

fn verify_password(password: &str, hashed: &str) -> Result<bool, AuthError> {
    verify(password, hashed).map_err(|err| AuthError::Internal(err.to_string()))
}

fn map_sqlx_error(err: sqlx::Error) -> AuthError {
    if let sqlx::Error::Database(ref db_err) = err
        && db_err.code().as_deref() == Some("23505")
    {
        return AuthError::AlreadyExists("email already in use".to_string());
    }
    AuthError::Internal(err.to_string())
}

#[cfg(test)]
#[path = "user_store_tests.rs"]
mod tests;
