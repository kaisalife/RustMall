use crate::AppError;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// 雪花算法配置
//可以用原子操作优化sequence的更新
const EPOCH: u64 = 1704067200000; // 2024-01-01 00:00:00 UTC
const WORKER_ID_BITS: u8 = 10;
const SEQUENCE_BITS: u8 = 12;

const MAX_WORKER_ID: u64 = (1 << WORKER_ID_BITS) - 1;
const MAX_SEQUENCE: u64 = (1 << SEQUENCE_BITS) - 1;

const WORKER_ID_SHIFT: u8 = SEQUENCE_BITS;
const TIMESTAMP_SHIFT: u8 = SEQUENCE_BITS + WORKER_ID_BITS;

pub struct SnowflakeIdGenerator {
    worker_id: u64,
    sequence: Mutex<SequenceState>,
}

struct SequenceState {
    sequence: u64,
    last_timestamp: u64,
}

impl SnowflakeIdGenerator {
    pub fn new(worker_id: u64) -> Result<Self, AppError> {
        if worker_id > MAX_WORKER_ID {
            return Err(AppError::IdGenerationError(
                "Worker ID exceeds maximum value".to_string(),
            ));
        }

        Ok(Self {
            worker_id,
            sequence: Mutex::new(SequenceState {
                sequence: 0,
                last_timestamp: 0,
            }),
        })
    }

    pub fn generate(&self) -> Result<u64, AppError> {
        let mut state = self
            .sequence
            .lock()
            .map_err(|_| AppError::IdGenerationError("Mutex lock failed".to_string()))?;

        let mut timestamp = Self::get_timestamp();

        if timestamp < state.last_timestamp {
            return Err(AppError::IdGenerationError(
                "Clock moved backwards, refusing to generate ID".to_string(),
            ));
        }

        if timestamp == state.last_timestamp {
            state.sequence = (state.sequence + 1) & MAX_SEQUENCE;
            if state.sequence == 0 {
                timestamp = Self::wait_next_millis(state.last_timestamp);
            }
        } else {
            state.sequence = 0;
        }

        state.last_timestamp = timestamp;

        let id = ((timestamp - EPOCH) << TIMESTAMP_SHIFT)
            | (self.worker_id << WORKER_ID_SHIFT)
            | state.sequence;

        Ok(id)
    }

    fn get_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64
    }

    fn wait_next_millis(last_timestamp: u64) -> u64 {
        let mut timestamp = Self::get_timestamp();
        while timestamp <= last_timestamp {
            std::thread::sleep(Duration::from_micros(100));
            timestamp = Self::get_timestamp();
        }
        timestamp
    }
}

impl Default for SnowflakeIdGenerator {
    fn default() -> Self {
        Self::new(1).expect("Failed to create default snowflake generator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_generate_unique_ids() {
        let generator = SnowflakeIdGenerator::new(1).unwrap();
        let mut ids = HashSet::new();

        for _ in 0..1000 {
            let id = generator.generate().unwrap();
            assert!(ids.insert(id), "Duplicate ID generated");
        }
    }

    #[test]
    fn test_invalid_worker_id() {
        assert!(SnowflakeIdGenerator::new(MAX_WORKER_ID + 1).is_err());
    }
}
