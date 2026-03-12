use cuckoofilter::CuckooFilter;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 统一的全网斩首广播主题。
pub const SLASH_TOPIC: &str = "/lifeplus/matrix/slashed/v1";

/// 赛博死亡证明，来自可信 slasher。
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DeathCertificate {
    pub rogue_agent_pubkey: String,
    pub reason: String,
    pub timestamp: i64,
    pub executioner_signature: Vec<u8>,
}

/// 使用布谷鸟过滤器的低内存黑名单屏障。
#[derive(Clone)]
pub struct QuarantineBarrier {
    inner: Arc<RwLock<CuckooFilter<DefaultHasher>>>,
}

impl Default for QuarantineBarrier {
    fn default() -> Self {
        Self::new()
    }
}

impl QuarantineBarrier {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CuckooFilter::new())),
        }
    }

    /// 将被斩首身份加入本地隔离区。
    pub async fn quarantine(&self, rogue_agent_pubkey: &str) {
        let _ = self
            .inner
            .write()
            .await
            .add(&rogue_agent_pubkey.to_string());
    }

    /// 在处理任何意图前做 O(1) 级别预检。
    pub async fn pre_flight_check(&self, agent_pubkey: &str) -> Result<(), String> {
        let is_slashed = self.inner.read().await.contains(&agent_pubkey.to_string());
        if is_slashed {
            return Err(
                "⛔ [CONNECTION REFUSED] You are legally dead in the Life++ Matrix. TCP connection dropped.".to_string(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn quarantined_identity_is_blocked() {
        let barrier = QuarantineBarrier::new();
        barrier.quarantine("rogue-alpha").await;

        let err = barrier
            .pre_flight_check("rogue-alpha")
            .await
            .expect_err("rogue identity should be blocked");
        assert!(err.contains("CONNECTION REFUSED"));
    }

    #[tokio::test]
    async fn unknown_identity_is_allowed() {
        let barrier = QuarantineBarrier::new();
        barrier
            .pre_flight_check("clean-beta")
            .await
            .expect("non-slashed identity should pass");
    }
}
