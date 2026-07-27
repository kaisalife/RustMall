use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub email: String,
    pub password_hash: String,
    pub nickname: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(id: u64, email: String, password_hash: String, nickname: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            email,
            password_hash,
            nickname,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_password(&mut self, new_password_hash: String) {
        self.password_hash = new_password_hash;
        self.updated_at = Utc::now();
    }

    pub fn update_nickname(&mut self, new_nickname: String) {
        self.nickname = new_nickname;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user() -> User {
        User::new(
            1,
            "test@example.com".to_string(),
            "old_hash".to_string(),
            "tester".to_string(),
        )
    }

    #[test]
    fn test_new_user() {
        let user = sample_user();
        assert_eq!(user.id, 1);
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password_hash, "old_hash");
        assert_eq!(user.nickname, "tester");
        assert_eq!(user.created_at, user.updated_at);
    }

    #[test]
    fn test_update_password() {
        let mut user = sample_user();
        assert_eq!(user.password_hash, "old_hash");
        user.update_password("new_hash".to_string());
        assert_eq!(user.password_hash, "new_hash");
    }

    #[test]
    fn test_update_nickname() {
        let mut user = sample_user();
        assert_eq!(user.nickname, "tester");
        user.update_nickname("new_nick".to_string());
        assert_eq!(user.nickname, "new_nick");
    }

    #[test]
    fn test_updated_at_changes() {
        let mut user = sample_user();
        let old_updated = user.updated_at;
        // Sleep to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(50));
        user.update_nickname("new_nick".to_string());
        assert!(user.updated_at > old_updated);
    }
}
