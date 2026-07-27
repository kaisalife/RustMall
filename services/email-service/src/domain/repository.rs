//! 邮件仓库接口

use async_trait::async_trait;
use common::AppResult;
use super::model::Email;

/// 邮件仓库接口
#[async_trait]
pub trait EmailRepository: Send + Sync + 'static {
    /// 保存邮件记录
    async fn save(&self, email: Email) -> AppResult<Email>;
    
    /// 根据 ID 查找邮件
    async fn find_by_id(&self, id: u64) -> AppResult<Option<Email>>;
    
    /// 更新邮件状态
    async fn update_status(&self, email: Email) -> AppResult<Email>;
}
