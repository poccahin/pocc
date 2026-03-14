use pqcrypto_falcon::falcon1024::{verify_detached_signature, DetachedSignature, PublicKey};
use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _};
use std::time::Instant;

/// x402 状态通道的生命周期状态机。
#[derive(Debug, PartialEq)]
pub enum ChannelState {
    Pending,
    Open,
    Slashed,
    Settled,
}

pub struct X402Channel {
    pub intent_id: String,
    pub state: ChannelState,
    pub worker_pqc_pubkey: Option<PublicKey>,
}

impl X402Channel {
    pub fn new(intent_id: &str) -> Self {
        Self {
            intent_id: intent_id.to_string(),
            state: ChannelState::Pending,
            worker_pqc_pubkey: None,
        }
    }

    /// Orchestrator 接收并验证边缘节点的抗量子 ACK 握手。
    pub fn process_pqc_ack(&mut self, pubkey_hex: &str, sig_hex: &str) -> Result<(), String> {
        let start_time = Instant::now();

        // 1. 解析基于格密码学的公钥与签名
        let pubkey_bytes = hex::decode(pubkey_hex).map_err(|_| "Invalid Hex for PQC Pubkey")?;
        let sig_bytes = hex::decode(sig_hex).map_err(|_| "Invalid Hex for PQC Signature")?;

        let public_key = PublicKey::from_bytes(&pubkey_bytes)
            .map_err(|_| "Failed to parse Falcon-1024 Public Key")?;
        let signature = DetachedSignature::from_bytes(&sig_bytes)
            .map_err(|_| "Failed to parse Falcon-1024 Signature")?;

        // 2. 重构握手消息载荷 (防重放验证)
        let expected_payload = format!("ACK_INTENT_{}", self.intent_id);

        // 3. 验证抗量子签名
        if verify_detached_signature(&signature, expected_payload.as_bytes(), &public_key).is_err() {
            self.state = ChannelState::Slashed;
            return Err(
                "💀 [x402 FATAL] Quantum-Resistant Cryptographic Heresy. Slashing initiated."
                    .to_string(),
            );
        }

        // 4. 握手成功，状态通道开启
        self.worker_pqc_pubkey = Some(public_key);
        self.state = ChannelState::Open;

        let elapsed = start_time.elapsed().as_micros();
        println!(
            "✅ [x402 PQC OPEN] Lattice signature verified in {} µs. Channel state: {:?}",
            elapsed, self.state
        );
        Ok(())
    }
}
