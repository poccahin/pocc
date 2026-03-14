use pqcrypto_falcon::falcon1024::{detached_sign, verify_detached_signature, DetachedSignature, PublicKey, SecretKey};
use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _};
use sha2::{Digest, Sha256};

/// 存在于内存中的微秒级 x402 状态通道（使用 Falcon-1024 抗量子签名）
#[derive(Debug, Clone)]
pub struct X402StateChannel {
    pub channel_id: String,
    pub orchestrator_pubkey: PublicKey,
    pub worker_pubkey: PublicKey,
    pub nonce: u64,                 // 极速递增的交易序号
    pub total_yield_life_plus: f64, // 当前累计结算金额
}

impl X402StateChannel {
    /// 微秒级高频更新 (纯内存操作，零网络 IO)
    /// 人型机器人每完成一个动作 (如毫秒级的电机反馈)，就执行一次微支付
    pub fn micro_settle(
        &mut self,
        micro_payment: f64,
        orchestrator_secret_key: &SecretKey,
    ) -> DetachedSignature {
        self.nonce += 1;
        self.total_yield_life_plus += micro_payment;

        // 生成不可伪造的状态快照
        let mut hasher = Sha256::new();
        hasher.update(self.channel_id.as_bytes());
        hasher.update(self.nonce.to_be_bytes());
        hasher.update(self.total_yield_life_plus.to_be_bytes());
        let state_hash = hasher.finalize();

        // Orchestrator 以 Falcon-1024 格密码学私钥对最新余额进行抗量子签名
        detached_sign(&state_hash, orchestrator_secret_key)
    }

    /// Worker 验证微支付是否有效（Falcon-1024 抗量子验签）
    pub fn verify_micro_payment(&self, signature: &DetachedSignature) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(self.channel_id.as_bytes());
        hasher.update(self.nonce.to_be_bytes());
        hasher.update(self.total_yield_life_plus.to_be_bytes());
        let state_hash = hasher.finalize();

        verify_detached_signature(signature, &state_hash, &self.orchestrator_pubkey).is_ok()
    }
}
