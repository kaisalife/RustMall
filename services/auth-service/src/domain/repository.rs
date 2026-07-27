use async_trait::async_trait;

use super::User;
use common::AppResult;

#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn create(&self, user: User) -> AppResult<User>;
    async fn find_by_id(&self, id: u64) -> AppResult<Option<User>>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
    async fn update(&self, user: User) -> AppResult<User>;
    async fn delete(&self, id: u64) -> AppResult<()>;
}
