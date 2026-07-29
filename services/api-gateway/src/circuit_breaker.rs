//! 熔断器实现
//!
//! 提供简单的连续失败熔断，保护后端服务免受级联故障。
//! 状态机：Closed（正常）-> Open（熔断）-> HalfOpen（半开探测）-> Closed/Open。
//!
//! # 使用方式
//! ```ignore
//! let cb = CircuitBreaker::new(5, Duration::from_secs(30));
//! if cb.can_proceed() {
//!     match some_rpc_call().await {
//!         Ok(v) => cb.record_success(),
//!         Err(e) => cb.record_failure(),
//!     }
//! }
//! ```

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 关闭状态（正常放行请求）
    Closed,
    /// 开启状态（拒绝请求）
    Open,
    /// 半开状态（放行探测请求）
    HalfOpen,
}

/// 熔断器内部状态（通过 Arc 共享，Clone 后共享同一实例）
#[derive(Debug)]
struct CircuitBreakerInner {
    /// 连续失败计数
    failure_count: AtomicU32,
    /// 当前状态
    state: Mutex<CircuitState>,
    /// 最后一次失败时间
    last_failure_time: Mutex<Option<Instant>>,
    /// 触发熔断的连续失败次数阈值
    threshold: u32,
    /// 熔断后多久尝试半开
    reset_timeout: Duration,
}

/// 简单连续失败熔断器
///
/// 连续失败达到阈值后熔断，reset_timeout 后进入半开状态尝试恢复。
/// Clone 后共享同一状态，适合在多处引用（如 GrpcClients 的各字段）。
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    inner: Arc<CircuitBreakerInner>,
}

impl CircuitBreaker {
    /// 创建熔断器
    ///
    /// - `threshold`: 连续失败多少次触发熔断
    /// - `reset_timeout`: 熔断后多久尝试半开
    pub fn new(threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(CircuitBreakerInner {
                failure_count: AtomicU32::new(0),
                state: Mutex::new(CircuitState::Closed),
                last_failure_time: Mutex::new(None),
                threshold,
                reset_timeout,
            }),
        }
    }

    /// 创建默认熔断器（连续失败 5 次，30 秒后半开）
    pub fn default_cb() -> Self {
        Self::new(5, Duration::from_secs(30))
    }

    /// 检查请求是否可以放行
    ///
    /// - Closed: 始终放行
    /// - Open: 超过 reset_timeout 后转为 HalfOpen 并放行，否则拒绝
    /// - HalfOpen: 放行（允许探测请求）
    pub fn can_proceed(&self) -> bool {
        let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        match *state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => {
                // 检查是否已过冷却期
                let last_failure = self
                    .inner
                    .last_failure_time
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if let Some(time) = *last_failure {
                    if time.elapsed() >= self.inner.reset_timeout {
                        // 冷却期已过，进入半开状态
                        *state = CircuitState::HalfOpen;
                        tracing::info!("Circuit breaker transitioning to HalfOpen");
                        return true;
                    }
                }
                false
            }
        }
    }

    /// 记录成功调用
    ///
    /// 重置失败计数，状态转为 Closed。
    pub fn record_success(&self) {
        self.inner.failure_count.store(0, Ordering::Relaxed);
        let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        if *state != CircuitState::Closed {
            tracing::info!("Circuit breaker transitioning to Closed (recovered)");
            *state = CircuitState::Closed;
        }
    }

    /// 记录失败调用
    ///
    /// 递增连续失败计数，达到阈值后转为 Open。
    /// 半开状态下失败则重新熔断。
    pub fn record_failure(&self) {
        let count = self.inner.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        {
            let mut last_failure = self
                .inner
                .last_failure_time
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *last_failure = Some(Instant::now());
        }
        let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        match *state {
            CircuitState::HalfOpen => {
                // 半开状态下探测失败，重新熔断
                tracing::warn!("Circuit breaker re-opening from HalfOpen (probe failed)");
                *state = CircuitState::Open;
            }
            CircuitState::Closed => {
                if count >= self.inner.threshold {
                    tracing::warn!(
                        "Circuit breaker opening after {} consecutive failures",
                        count
                    );
                    *state = CircuitState::Open;
                }
            }
            CircuitState::Open => {
                // 已是 Open，保持不变
            }
        }
    }

    /// 获取当前状态（用于监控和测试）
    pub fn state(&self) -> CircuitState {
        *self.inner.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_stays_closed_under_threshold() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(30));
        assert_eq!(cb.state(), CircuitState::Closed);

        for _ in 0..4 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_proceed());
    }

    #[test]
    fn test_circuit_opens_at_threshold() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(30));

        for _ in 0..5 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_proceed());
    }

    #[test]
    fn test_success_resets_failure_count() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(30));

        for _ in 0..4 {
            cb.record_failure();
        }
        cb.record_success();
        // 失败计数被重置，再失败 4 次不应熔断
        for _ in 0..4 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_after_timeout() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(50));

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_proceed());

        // 等待冷却期
        std::thread::sleep(Duration::from_millis(60));
        assert!(cb.can_proceed());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_success_closes_circuit() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(50));

        cb.record_failure();
        std::thread::sleep(Duration::from_millis(60));
        cb.can_proceed(); // 触发转为 HalfOpen

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_failure_reopens_circuit() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(50));

        cb.record_failure();
        std::thread::sleep(Duration::from_millis(60));
        cb.can_proceed(); // 触发转为 HalfOpen

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_clone_shares_state() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(30));
        let cb2 = cb.clone();

        cb.record_failure();
        cb2.record_failure();
        cb.record_failure();
        cb2.record_failure();
        cb.record_failure();

        // 5 次失败通过两个 clone 分别记录，状态共享
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb2.state(), CircuitState::Open);
    }
}
