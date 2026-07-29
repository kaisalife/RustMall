use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::time::Duration;

#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
}

impl RedisCache {
    pub async fn new(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn })
    }

    pub async fn get_string(&self, key: &str) -> Result<Option<String>, redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.get(key).await
    }

    pub async fn set_string(
        &self,
        key: &str,
        value: &str,
        ttl: Duration,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.set_ex(key, value, ttl.as_secs()).await
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Box<dyn std::error::Error + Send + Sync>> {
        match self.get_string(key).await? {
            Some(val) => Ok(Some(serde_json::from_str(&val)?)),
            None => Ok(None),
        }
    }

    pub async fn set_json<T: serde::Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string(value)?;
        self.set_string(key, &json, ttl).await?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.del(key).await
    }
}
