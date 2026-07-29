use sqlx::{query, query_as, FromRow, PgPool};

use crate::domain::{User, UserRepository};
use common::AppResult;

#[derive(Clone, FromRow)]
struct UserRecord {
    id: i64,
    email: String,
    password_hash: String,
    nickname: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl UserRecord {
    fn into_domain(self) -> User {
        User {
            id: self.id as u64,
            email: self.email,
            password_hash: self.password_hash,
            nickname: self.nickname,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Clone)]
pub struct UserRepositoryImpl {
    pool: PgPool,
}

impl UserRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn create(&self, user: User) -> AppResult<User> {
        let record = query_as::<_, UserRecord>(
            r#"
            INSERT INTO users (id, email, password_hash, nickname, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, password_hash, nickname, created_at, updated_at
            "#,
        )
        .bind(user.id as i64)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.nickname)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn find_by_id(&self, id: u64) -> AppResult<Option<User>> {
        let record = query_as::<_, UserRecord>(
            r#"
            SELECT id, email, password_hash, nickname, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.into_domain()))
    }

    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let record = query_as::<_, UserRecord>(
            r#"
            SELECT id, email, password_hash, nickname, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.into_domain()))
    }

    async fn update(&self, user: User) -> AppResult<User> {
        let record = query_as::<_, UserRecord>(
            r#"
            UPDATE users
            SET email = $2, password_hash = $3, nickname = $4, updated_at = $5
            WHERE id = $1
            RETURNING id, email, password_hash, nickname, created_at, updated_at
            "#,
        )
        .bind(user.id as i64)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.nickname)
        .bind(user.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.into_domain())
    }

    async fn delete(&self, id: u64) -> AppResult<()> {
        query(
            r#"
            DELETE FROM users WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
