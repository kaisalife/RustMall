use std::time::Duration;
use crate::{AppResult, AppError};

/// 对异步操作执行指数退避重试
///
/// # 参数
/// - `max_retries`: 最大重试次数（不含首次执行）
/// - `initial_delay`: 初始延迟
/// - `operation`: 异步操作闭包
pub async fn retry_with_backoff<F, Fut, T>(
    max_retries: u32,
    initial_delay: Duration,
    operation: F,
) -> AppResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = AppResult<T>>,
{
    let mut delay = initial_delay;
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                // 判断是否为可重试的错误（数据库连接错误）
                let is_retryable = matches!(&e, AppError::Database(_));

                if !is_retryable || attempt == max_retries {
                    return Err(e);
                }

                tracing::warn!(
                    attempt = attempt + 1,
                    max_retries = max_retries + 1,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "Retrying after transient error"
                );

                tokio::time::sleep(delay).await;
                delay = delay * 2; // 指数退避
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AppError::internal("Retry exhausted")))
}

/// 默认重试策略：3 次重试，初始延迟 100ms
pub async fn retry_db<F, Fut, T>(operation: F) -> AppResult<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = AppResult<T>>,
{
    retry_with_backoff(3, Duration::from_millis(100), operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_succeeds_first_try() {
        let counter = Arc::new(AtomicU32::new(0));

        let result: AppResult<u32> = retry_with_backoff(
            3,
            Duration::from_millis(1),
            || {
                let c = counter.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(42u32)
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_retry() {
        let counter = Arc::new(AtomicU32::new(0));

        let result: AppResult<u32> = retry_with_backoff(
            3,
            Duration::from_millis(1),
            || {
                let c = counter.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n == 0 {
                        Err(AppError::Database(sqlx::Error::Io(
                            std::io::Error::new(
                                std::io::ErrorKind::ConnectionRefused,
                                "connection refused",
                            ),
                        )))
                    } else {
                        Ok(42u32)
                    }
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let counter = Arc::new(AtomicU32::new(0));

        let result: AppResult<u32> = retry_with_backoff(
            2,
            Duration::from_millis(1),
            || {
                let c = counter.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(AppError::Database(sqlx::Error::Io(
                        std::io::Error::new(
                            std::io::ErrorKind::ConnectionRefused,
                            "connection refused",
                        ),
                    )))
                }
            },
        )
        .await;

        assert!(result.is_err());
        // 1 initial + 2 retries = 3 attempts
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_non_database_error_not_retried() {
        let counter = Arc::new(AtomicU32::new(0));

        let result: AppResult<u32> = retry_with_backoff(
            3,
            Duration::from_millis(1),
            || {
                let c = counter.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(AppError::internal("non-retryable error"))
                }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
