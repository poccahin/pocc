use ed25519_dalek::{Signature, Verifier, VerifyingKey};
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
    pub worker_pubkey: Option<VerifyingKey>,
}

impl X402Channel {
    pub fn new(intent_id: &str) -> Self {
        Self {
            intent_id: intent_id.to_string(),
            state: ChannelState::Pending,
            worker_pubkey: None,
        }
    }

    /// Orchestrator 接收并验证边缘节点的 ACK 握手。
    pub fn process_ack(&mut self, pubkey_hex: &str, sig_hex: &str) -> Result<(), String> {
        let start_time = Instant::now();

        // 1. 解析密码学原语
        let pubkey_bytes = hex::decode(pubkey_hex).map_err(|_| "Invalid Hex for Pubkey")?;
        let sig_bytes = hex::decode(sig_hex).map_err(|_| "Invalid Hex for Signature")?;

        let verifying_key = VerifyingKey::try_from(pubkey_bytes.as_slice())
            .map_err(|_| "Failed to parse Ed25519 Public Key")?;
        let signature = Signature::from_slice(&sig_bytes)
            .map_err(|_| "Failed to parse Signature")?;

        // 2. 重构握手消息载荷 (防重放验证)
        let expected_payload = format!("ACK_INTENT_{}", self.intent_id);

        // 3. O(1) 极速验证数学签名
        if verifying_key
            .verify(expected_payload.as_bytes(), &signature)
            .is_err()
        {
            self.state = ChannelState::Slashed;
            return Err(
                "💀 [x402 FATAL] Cryptographic Heresy. Signature invalid. Slashing initiated."
                    .to_string(),
            );
        }

        // 4. 握手成功，状态通道开启
        self.worker_pubkey = Some(verifying_key);
        self.state = ChannelState::Open;

        let elapsed = start_time.elapsed().as_micros();
        println!(
            "✅ [x402 OPEN] Signature verified in {} µs. Channel state: {:?}",
            elapsed, self.state
        );
        Ok(())
    }
}
