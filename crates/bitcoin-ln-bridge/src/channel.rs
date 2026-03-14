//! BTC 计价双向支付通道
//!
//! 实现 Bitcoin Lightning Network 支付通道的核心状态机（参考 BOLT-2）：
//!
//! ```text
//!  [链上开通] ──► Open ──► [Active 微支付] ──► Closing ──► [链上结算]
//!                │                                 ▲
//!                └──── Slashed ──────────────────┘ (单方强制关闭)
//! ```
//!
//! ## 关键设计
//! - **余额单位**: 聪（Satoshi）— 与比特币链完全一致
//! - **HTLC 超时**: `expiry_blocks`（相对区块数），典型值 144（≈24h）
//! - **防重放**: `commitment_number` 单调递增，对应 BOLT-2 的 commitment_signed

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::invoice::Satoshi;

#[derive(Error, Debug, PartialEq)]
pub enum ChannelError {
    #[error("Channel is not open (state: {0})")]
    NotOpen(String),
    #[error("Insufficient balance: {holder} has {available} sat, needs {needed} sat")]
    InsufficientBalance { holder: String, available: Satoshi, needed: Satoshi },
    #[error("HTLC not found or preimage mismatch")]
    HtlcError,
    #[error("Commitment number overflow")]
    CommitmentOverflow,
}

/// 支付通道生命周期
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelState {
    /// 通道开启，双方可发起 HTLC
    Open,
    /// 合作关闭中，等待链上确认
    Closing,
    /// 已关闭，余额已锁定为最终状态
    Closed,
    /// 单方强制关闭，可能触发惩罚机制
    Slashed,
}

impl std::fmt::Display for ChannelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelState::Open => write!(f, "Open"),
            ChannelState::Closing => write!(f, "Closing"),
            ChannelState::Closed => write!(f, "Closed"),
            ChannelState::Slashed => write!(f, "Slashed"),
        }
    }
}

/// 挂起中的 HTLC（哈希时间锁合约）
///
/// 对应 BOLT-2 的 `update_add_htlc` 消息。
#[derive(Debug, Clone)]
pub struct LnHtlc {
    /// HTLC ID（通道内单调递增）
    pub htlc_id: u64,
    /// SHA-256(preimage)
    pub payment_hash: [u8; 32],
    /// 锁定金额（聪）
    pub amount_sat: Satoshi,
    /// 超时区块数（相对）；过期后发送方可 `fail_htlc`
    pub expiry_blocks: u32,
    /// 接收方节点 ID / DID
    pub recipient_id: String,
}

impl LnHtlc {
    /// 用原像构造 HTLC，payment_hash = SHA-256(preimage)
    pub fn new(
        htlc_id: u64,
        preimage: &[u8; 32],
        amount_sat: Satoshi,
        expiry_blocks: u32,
        recipient_id: &str,
    ) -> Self {
        let mut h = Sha256::new();
        h.update(preimage);
        let mut payment_hash = [0u8; 32];
        payment_hash.copy_from_slice(&h.finalize());
        LnHtlc { htlc_id, payment_hash, amount_sat, expiry_blocks, recipient_id: recipient_id.to_string() }
    }

    /// 验证原像是否匹配 payment_hash
    pub fn verify_preimage(&self, preimage: &[u8; 32]) -> bool {
        let mut h = Sha256::new();
        h.update(preimage);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&h.finalize());
        hash == self.payment_hash
    }
}

/// 已关闭通道的最终余额快照
#[derive(Debug, Clone)]
pub struct ChannelCloseRecord {
    pub channel_id: String,
    pub local_node_id: String,
    pub remote_node_id: String,
    pub local_final_sat: Satoshi,
    pub remote_final_sat: Satoshi,
}

/// BTC 计价双向支付通道
///
/// 模拟 BOLT-2 的双向承诺交易（Commitment Transaction）状态机：
/// 每次 HTLC 更新对应一个新的 `commitment_number`。
#[derive(Debug)]
pub struct BtcLightningChannel {
    pub channel_id: String,
    /// 本地节点 ID（开通方，对应 BOLT-2 的 funder）
    pub local_node_id: String,
    /// 远端节点 ID
    pub remote_node_id: String,
    /// 本地持有余额（聪）
    pub local_balance_sat: Satoshi,
    /// 远端持有余额（聪）
    pub remote_balance_sat: Satoshi,
    /// 单调递增承诺号（BOLT-2 commitment number）
    pub commitment_number: u64,
    /// 下一个 HTLC 的 ID
    next_htlc_id: u64,
    pub state: ChannelState,
    /// 本地方锁定的待处理 HTLC 列表
    pending_htlcs: Vec<LnHtlc>,
}

impl BtcLightningChannel {
    /// 开启一条新通道
    ///
    /// - `local_push_sat` — 本地初始余额（来自链上 funding_tx）
    /// - `remote_push_sat` — 远端初始余额（push_msat 机制）
    pub fn open(
        local_node_id: &str,
        remote_node_id: &str,
        local_push_sat: Satoshi,
        remote_push_sat: Satoshi,
    ) -> Self {
        let channel_id = Self::derive_channel_id(local_node_id, remote_node_id);
        BtcLightningChannel {
            channel_id,
            local_node_id: local_node_id.to_string(),
            remote_node_id: remote_node_id.to_string(),
            local_balance_sat: local_push_sat,
            remote_balance_sat: remote_push_sat,
            commitment_number: 0,
            next_htlc_id: 0,
            state: ChannelState::Open,
            pending_htlcs: Vec::new(),
        }
    }

    /// 本地方向远端发出 HTLC（对应 `update_add_htlc`）
    pub fn add_htlc(&mut self, preimage: &[u8; 32], amount_sat: Satoshi, expiry_blocks: u32, recipient_id: &str) -> Result<u64, ChannelError> {
        self.require_open()?;
        if self.local_balance_sat < amount_sat {
            return Err(ChannelError::InsufficientBalance {
                holder: self.local_node_id.clone(),
                available: self.local_balance_sat,
                needed: amount_sat,
            });
        }
        let htlc_id = self.next_htlc_id;
        self.next_htlc_id += 1;
        self.local_balance_sat -= amount_sat;
        self.commitment_number = self.commitment_number.checked_add(1)
            .ok_or(ChannelError::CommitmentOverflow)?;

        let htlc = LnHtlc::new(htlc_id, preimage, amount_sat, expiry_blocks, recipient_id);
        self.pending_htlcs.push(htlc);
        Ok(htlc_id)
    }

    /// 接收方揭示原像，结算 HTLC（对应 `update_fulfill_htlc`）
    ///
    /// 成功后远端余额增加。
    pub fn fulfill_htlc(&mut self, preimage: &[u8; 32]) -> Result<Satoshi, ChannelError> {
        self.require_open()?;
        let pos = self.pending_htlcs.iter().position(|h| h.verify_preimage(preimage))
            .ok_or(ChannelError::HtlcError)?;
        let htlc = self.pending_htlcs.remove(pos);
        self.remote_balance_sat += htlc.amount_sat;
        self.commitment_number += 1;
        Ok(htlc.amount_sat)
    }

    /// 超时/失败，退款给本地方（对应 `update_fail_htlc`）
    pub fn fail_htlc(&mut self, preimage: &[u8; 32]) -> Result<Satoshi, ChannelError> {
        self.require_open()?;
        let pos = self.pending_htlcs.iter().position(|h| h.verify_preimage(preimage))
            .ok_or(ChannelError::HtlcError)?;
        let htlc = self.pending_htlcs.remove(pos);
        self.local_balance_sat += htlc.amount_sat;
        self.commitment_number += 1;
        Ok(htlc.amount_sat)
    }

    /// 合作关闭通道（`shutdown` + `closing_signed`），返回最终余额记录
    pub fn cooperative_close(&mut self) -> Result<ChannelCloseRecord, ChannelError> {
        self.require_open()?;
        self.state = ChannelState::Closed;
        Ok(ChannelCloseRecord {
            channel_id: self.channel_id.clone(),
            local_node_id: self.local_node_id.clone(),
            remote_node_id: self.remote_node_id.clone(),
            local_final_sat: self.local_balance_sat,
            remote_final_sat: self.remote_balance_sat,
        })
    }

    /// 总容量（聪）= 本地 + 远端（不含挂起 HTLC 中的金额）
    pub fn total_capacity_sat(&self) -> Satoshi {
        self.local_balance_sat
            + self.remote_balance_sat
            + self.pending_htlcs.iter().map(|h| h.amount_sat).sum::<u64>()
    }

    fn require_open(&self) -> Result<(), ChannelError> {
        if self.state != ChannelState::Open {
            Err(ChannelError::NotOpen(self.state.to_string()))
        } else {
            Ok(())
        }
    }

    /// 通道 ID = SHA-256(sorted(local, remote))[:8] 的十六进制
    pub fn derive_channel_id(a: &str, b: &str) -> String {
        let (x, y) = if a < b { (a, b) } else { (b, a) };
        let mut h = Sha256::new();
        h.update(x.as_bytes());
        h.update(b"|");
        h.update(y.as_bytes());
        hex::encode(&h.finalize()[..8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREIMAGE: [u8; 32] = [0x11; 32];
    const PREIMAGE2: [u8; 32] = [0x22; 32];

    #[test]
    fn open_channel_initial_balances() {
        let ch = BtcLightningChannel::open("alice", "bob", 1_000_000, 500_000);
        assert_eq!(ch.local_balance_sat, 1_000_000);
        assert_eq!(ch.remote_balance_sat, 500_000);
        assert_eq!(ch.state, ChannelState::Open);
        assert_eq!(ch.total_capacity_sat(), 1_500_000);
    }

    #[test]
    fn add_and_fulfill_htlc() {
        let mut ch = BtcLightningChannel::open("alice", "bob", 1_000_000, 0);
        ch.add_htlc(&PREIMAGE, 50_000, 144, "bob").unwrap();
        assert_eq!(ch.local_balance_sat, 950_000);
        assert_eq!(ch.pending_htlcs.len(), 1);

        let earned = ch.fulfill_htlc(&PREIMAGE).unwrap();
        assert_eq!(earned, 50_000);
        assert_eq!(ch.remote_balance_sat, 50_000);
        assert!(ch.pending_htlcs.is_empty());
    }

    #[test]
    fn fail_htlc_refunds_local() {
        let mut ch = BtcLightningChannel::open("alice", "bob", 1_000_000, 0);
        ch.add_htlc(&PREIMAGE, 100_000, 144, "bob").unwrap();
        ch.fail_htlc(&PREIMAGE).unwrap();
        assert_eq!(ch.local_balance_sat, 1_000_000);
        assert!(ch.pending_htlcs.is_empty());
    }

    #[test]
    fn insufficient_balance_error() {
        let mut ch = BtcLightningChannel::open("alice", "bob", 1_000, 0);
        let err = ch.add_htlc(&PREIMAGE, 5_000, 144, "bob").unwrap_err();
        assert!(matches!(err, ChannelError::InsufficientBalance { .. }));
    }

    #[test]
    fn cooperative_close() {
        let mut ch = BtcLightningChannel::open("alice", "bob", 800_000, 200_000);
        ch.add_htlc(&PREIMAGE, 100_000, 144, "bob").unwrap();
        ch.fulfill_htlc(&PREIMAGE).unwrap();
        let rec = ch.cooperative_close().unwrap();
        assert_eq!(rec.local_final_sat, 700_000);
        assert_eq!(rec.remote_final_sat, 300_000);
        assert_eq!(ch.state, ChannelState::Closed);
    }

    #[test]
    fn commitment_number_increments() {
        let mut ch = BtcLightningChannel::open("alice", "bob", 1_000_000, 0);
        let n0 = ch.commitment_number;
        ch.add_htlc(&PREIMAGE, 1_000, 144, "bob").unwrap();
        assert_eq!(ch.commitment_number, n0 + 1);
        ch.fulfill_htlc(&PREIMAGE).unwrap();
        assert_eq!(ch.commitment_number, n0 + 2);
    }

    #[test]
    fn wrong_preimage_fails() {
        let mut ch = BtcLightningChannel::open("alice", "bob", 1_000_000, 0);
        ch.add_htlc(&PREIMAGE, 1_000, 144, "bob").unwrap();
        assert!(matches!(ch.fulfill_htlc(&PREIMAGE2).unwrap_err(), ChannelError::HtlcError));
    }
}
