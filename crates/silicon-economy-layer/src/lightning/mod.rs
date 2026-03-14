//! GOAT 闪电网络 — Life++ 智能体间协作支付层
//!
//! 灵感来源：GOAT-Hackathon-2026（eigmax/GOAT-Hackathon-2026）提出的
//! AI Agent 闪电通道网络。本模块将其思想映射到 Life++ 的 x402 状态通道体系：
//!
//! ```text
//!  Agent A ──[Channel A-B]──► Agent B
//!               │                 │
//!         HTLC Payment      HTLC Forward
//!               │                 │
//!            Agent C ──[Channel C-D]──► Agent D
//! ```
//!
//! 核心流程
//! --------
//! 1. **开通道** (`open_channel`)：两个 Life++ 智能体锁定 LIFE++ 担保金，
//!    建立双向余额账本。
//! 2. **HTLC 支付** (`send_htlc`)：发送方锁定金额并附加哈希锁 + 时间锁，
//!    接收方凭原像（preimage）解锁资金，实现原子性跨通道转账。
//! 3. **多跳路由** (`find_route` / `forward_htlc`)：`LightningChannelRouter`
//!    在 AHIN CR+ 引力图上做最短路径搜索，将支付拆解为多跳 HTLC 链。
//! 4. **关闭与结算** (`close_channel`)：将最终余额差额提交至
//!    `DailyNettingProcessor` 进行多边净额轧账，再批量上链结算。

use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

// ─────────────────────────────────────────────────────────────
// 错误类型
// ─────────────────────────────────────────────────────────────

#[derive(Error, Debug, PartialEq)]
pub enum LightningError {
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),
    #[error("Insufficient channel balance for {did}: needs {needed:.4}, has {available:.4}")]
    InsufficientBalance { did: String, needed: f64, available: f64 },
    #[error("HTLC expired or preimage mismatch")]
    HtlcFailure,
    #[error("No route found from {src} to {dst}")]
    NoRoute { src: String, dst: String },
    #[error("Channel already exists between {0} and {1}")]
    ChannelAlreadyExists(String, String),
    #[error("Channel is not open")]
    ChannelNotOpen,
}

// ─────────────────────────────────────────────────────────────
// 状态通道
// ─────────────────────────────────────────────────────────────

/// 双向闪电通道的生命周期状态
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelState {
    /// 通道已开启，双方余额有效
    Open,
    /// 已提交合作关闭请求，等待链上确认
    Closing,
    /// 通道已关闭，最终余额已提交结算
    Closed,
}

/// 一笔挂起的 HTLC（哈希时间锁合约）
#[derive(Debug, Clone)]
pub struct HtlcPayment {
    /// SHA-256(preimage) — 接收方凭原像解锁
    pub payment_hash: [u8; 32],
    /// 锁定金额（LIFE++）
    pub amount: f64,
    /// 相对超时（区块数）；超过此值则退款发送方
    pub expiry_blocks: u64,
    /// 目标接收方 DID
    pub recipient_did: String,
}

impl HtlcPayment {
    /// 用原像（preimage）构造一个 HTLC，`payment_hash = SHA-256(preimage)`
    pub fn new(preimage: &[u8], amount: f64, expiry_blocks: u64, recipient_did: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let mut payment_hash = [0u8; 32];
        payment_hash.copy_from_slice(&hasher.finalize());

        HtlcPayment {
            payment_hash,
            amount,
            expiry_blocks,
            recipient_did: recipient_did.to_string(),
        }
    }

    /// 验证原像是否匹配哈希锁
    pub fn verify_preimage(&self, preimage: &[u8]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hasher.finalize());
        hash == self.payment_hash
    }
}

/// 两个 Life++ 智能体之间的双向状态通道
#[derive(Debug, Clone)]
pub struct AgentLightningChannel {
    pub channel_id: String,
    /// 本地智能体 DID（通道发起方）
    pub local_did: String,
    /// 远端智能体 DID（通道接受方）
    pub remote_did: String,
    /// 本地余额（local 可向 remote 发送的最大金额）
    pub local_balance: f64,
    /// 远端余额（remote 可向 local 发送的最大金额）
    pub remote_balance: f64,
    /// 更新计数器（单调递增，防重放）
    pub nonce: u64,
    pub state: ChannelState,
    /// 挂起中的 HTLC 列表（链式转发时暂存）
    pending_htlcs: Vec<HtlcPayment>,
}

impl AgentLightningChannel {
    /// 开启一条新通道，`local_capacity` 为本地方锁定金额，`remote_capacity` 为对方锁定金额
    pub fn open(
        local_did: &str,
        remote_did: &str,
        local_capacity: f64,
        remote_capacity: f64,
    ) -> Self {
        let channel_id = Self::derive_id(local_did, remote_did);
        AgentLightningChannel {
            channel_id,
            local_did: local_did.to_string(),
            remote_did: remote_did.to_string(),
            local_balance: local_capacity,
            remote_balance: remote_capacity,
            nonce: 0,
            state: ChannelState::Open,
            pending_htlcs: Vec::new(),
        }
    }

    /// 向远端发送 HTLC，锁定本地余额
    pub fn add_htlc(&mut self, htlc: HtlcPayment) -> Result<(), LightningError> {
        if self.state != ChannelState::Open {
            return Err(LightningError::ChannelNotOpen);
        }
        if self.local_balance < htlc.amount {
            return Err(LightningError::InsufficientBalance {
                did: self.local_did.clone(),
                needed: htlc.amount,
                available: self.local_balance,
            });
        }
        self.local_balance -= htlc.amount;
        self.nonce += 1;
        self.pending_htlcs.push(htlc);
        Ok(())
    }

    /// 接收方提供原像，解锁 HTLC，完成微支付
    pub fn settle_htlc(&mut self, preimage: &[u8]) -> Result<f64, LightningError> {
        let pos = self
            .pending_htlcs
            .iter()
            .position(|h| h.verify_preimage(preimage))
            .ok_or(LightningError::HtlcFailure)?;

        let htlc = self.pending_htlcs.remove(pos);
        self.remote_balance += htlc.amount;
        self.nonce += 1;
        Ok(htlc.amount)
    }

    /// 超时回滚：退还锁定的 HTLC 金额给本地方
    pub fn fail_htlc(&mut self, preimage: &[u8]) -> Result<(), LightningError> {
        let pos = self
            .pending_htlcs
            .iter()
            .position(|h| h.verify_preimage(preimage))
            .ok_or(LightningError::HtlcFailure)?;

        let htlc = self.pending_htlcs.remove(pos);
        self.local_balance += htlc.amount;
        self.nonce += 1;
        Ok(())
    }

    /// 协作关闭通道，返回 `(local_final, remote_final)`
    pub fn cooperative_close(&mut self) -> (f64, f64) {
        self.state = ChannelState::Closed;
        (self.local_balance, self.remote_balance)
    }

    /// 通道 ID = SHA-256(sorted(local_did, remote_did)) 的前 16 字节十六进制
    fn derive_id(a: &str, b: &str) -> String {
        let (x, y) = if a < b { (a, b) } else { (b, a) };
        let mut hasher = Sha256::new();
        hasher.update(x.as_bytes());
        hasher.update(b"|");
        hasher.update(y.as_bytes());
        let hash = hasher.finalize();
        hex::encode(&hash[..8])
    }
}

// ─────────────────────────────────────────────────────────────
// 多跳路由器（GOAT 闪电图）
// ─────────────────────────────────────────────────────────────

/// 最终结算记录，输入给 `DailyNettingProcessor`
#[derive(Debug, Clone)]
pub struct ChannelSettlementRecord {
    pub local_did: String,
    pub remote_did: String,
    /// `local_final − local_opening`，正值表示 local 净赚，负值表示净付
    pub net_delta: f64,
}

/// GOAT 式智能体闪电网络路由器
///
/// 维护一张以智能体 DID 为节点、`AgentLightningChannel` 为边的无向图，
/// 支持多跳 HTLC 路由（基于 BFS 最短跳数）。
pub struct LightningChannelRouter {
    /// channel_id → channel
    channels: HashMap<String, AgentLightningChannel>,
    /// did → 所有相邻通道 ID（邻接表）
    adjacency: HashMap<String, Vec<String>>,
}

impl Default for LightningChannelRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl LightningChannelRouter {
    pub fn new() -> Self {
        LightningChannelRouter {
            channels: HashMap::new(),
            adjacency: HashMap::new(),
        }
    }

    /// 在两个智能体之间开启一条闪电通道
    pub fn open_channel(
        &mut self,
        local_did: &str,
        remote_did: &str,
        local_capacity: f64,
        remote_capacity: f64,
    ) -> Result<String, LightningError> {
        let channel_id = AgentLightningChannel::derive_id(local_did, remote_did);
        if self.channels.contains_key(&channel_id) {
            return Err(LightningError::ChannelAlreadyExists(
                local_did.to_string(),
                remote_did.to_string(),
            ));
        }
        let channel =
            AgentLightningChannel::open(local_did, remote_did, local_capacity, remote_capacity);
        self.adjacency
            .entry(local_did.to_string())
            .or_default()
            .push(channel_id.clone());
        self.adjacency
            .entry(remote_did.to_string())
            .or_default()
            .push(channel_id.clone());
        self.channels.insert(channel_id.clone(), channel);
        Ok(channel_id)
    }

    /// BFS 寻找从 `src_did` 到 `dst_did` 的最短路径（DID 序列）
    pub fn find_route(
        &self,
        src_did: &str,
        dst_did: &str,
        amount: f64,
    ) -> Result<Vec<String>, LightningError> {
        if src_did == dst_did {
            return Ok(vec![src_did.to_string()]);
        }

        // BFS: 队列中存储 (当前节点, 路径)
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();
        let mut visited: HashMap<String, bool> = HashMap::new();

        queue.push_back((src_did.to_string(), vec![src_did.to_string()]));
        visited.insert(src_did.to_string(), true);

        while let Some((current, path)) = queue.pop_front() {
            let channel_ids = match self.adjacency.get(&current) {
                Some(ids) => ids,
                None => continue,
            };

            for cid in channel_ids {
                let ch = match self.channels.get(cid) {
                    Some(c) if c.state == ChannelState::Open => c,
                    _ => continue,
                };

                let neighbor = if ch.local_did == current {
                    // 检查 local → remote 方向的流动性
                    if ch.local_balance < amount {
                        continue;
                    }
                    ch.remote_did.clone()
                } else {
                    // 检查 remote → local 方向的流动性
                    if ch.remote_balance < amount {
                        continue;
                    }
                    ch.local_did.clone()
                };

                if visited.contains_key(&neighbor) {
                    continue;
                }

                let mut new_path = path.clone();
                new_path.push(neighbor.clone());

                if neighbor == dst_did {
                    return Ok(new_path);
                }

                visited.insert(neighbor.clone(), true);
                queue.push_back((neighbor, new_path));
            }
        }

        Err(LightningError::NoRoute {
            src: src_did.to_string(),
            dst: dst_did.to_string(),
        })
    }

    /// 沿路径逐跳发送 HTLC（洋葱路由简化版）
    ///
    /// `route` 为 `find_route` 返回的 DID 列表，`preimage` 为支付原像。
    pub fn send_payment_along_route(
        &mut self,
        route: &[String],
        preimage: &[u8],
        amount: f64,
        expiry_blocks: u64,
    ) -> Result<(), LightningError> {
        if route.len() < 2 {
            return Ok(());
        }

        // 逐跳加 HTLC
        for window in route.windows(2) {
            let (from, to) = (&window[0], &window[1]);
            let channel_id = AgentLightningChannel::derive_id(from, to);
            let ch = self
                .channels
                .get_mut(&channel_id)
                .ok_or_else(|| LightningError::ChannelNotFound(channel_id.clone()))?;

            let htlc = HtlcPayment::new(preimage, amount, expiry_blocks, to);

            // 确保 HTLC 从正确方向扣款
            if ch.local_did == *from {
                ch.add_htlc(htlc)?;
            } else {
                // 反向通道：交换 local/remote 视角
                if ch.remote_balance < amount {
                    return Err(LightningError::InsufficientBalance {
                        did: from.clone(),
                        needed: amount,
                        available: ch.remote_balance,
                    });
                }
                ch.remote_balance -= amount;
                ch.nonce += 1;
                ch.pending_htlcs.push(htlc);
            }
        }

        // 接收方揭示原像，逆向结算所有 HTLC
        for window in route.windows(2).rev() {
            let (from, to) = (&window[0], &window[1]);
            let channel_id = AgentLightningChannel::derive_id(from, to);
            let ch = self
                .channels
                .get_mut(&channel_id)
                .ok_or_else(|| LightningError::ChannelNotFound(channel_id.clone()))?;

            if ch.local_did == *from {
                ch.settle_htlc(preimage)?;
            } else {
                // 反向：结算到 local
                let pos = ch
                    .pending_htlcs
                    .iter()
                    .position(|h| h.verify_preimage(preimage))
                    .ok_or(LightningError::HtlcFailure)?;
                let htlc = ch.pending_htlcs.remove(pos);
                ch.local_balance += htlc.amount;
                ch.nonce += 1;
            }
        }

        Ok(())
    }

    /// 关闭通道并返回结算记录，供 `DailyNettingProcessor` 消费
    pub fn close_channel(
        &mut self,
        channel_id: &str,
        opening_local_balance: f64,
    ) -> Result<ChannelSettlementRecord, LightningError> {
        let ch = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| LightningError::ChannelNotFound(channel_id.to_string()))?;

        let (local_final, _remote_final) = ch.cooperative_close();

        Ok(ChannelSettlementRecord {
            local_did: ch.local_did.clone(),
            remote_did: ch.remote_did.clone(),
            net_delta: local_final - opening_local_balance,
        })
    }

    /// 只读访问通道
    pub fn get_channel(&self, channel_id: &str) -> Option<&AgentLightningChannel> {
        self.channels.get(channel_id)
    }
}

// ─────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const PREIMAGE: &[u8] = b"life_plus_plus_secret_42";

    // ── HtlcPayment ──────────────────────────────────────────

    #[test]
    fn htlc_preimage_roundtrip() {
        let htlc = HtlcPayment::new(PREIMAGE, 1.0, 100, "agent-B.ahin.io");
        assert!(htlc.verify_preimage(PREIMAGE));
        assert!(!htlc.verify_preimage(b"wrong_preimage"));
    }

    // ── AgentLightningChannel ────────────────────────────────

    #[test]
    fn open_channel_balances() {
        let ch = AgentLightningChannel::open("A", "B", 10.0, 5.0);
        assert_eq!(ch.local_balance, 10.0);
        assert_eq!(ch.remote_balance, 5.0);
        assert_eq!(ch.state, ChannelState::Open);
    }

    #[test]
    fn add_and_settle_htlc() {
        let mut ch = AgentLightningChannel::open("A", "B", 10.0, 0.0);
        let htlc = HtlcPayment::new(PREIMAGE, 3.0, 100, "B");
        ch.add_htlc(htlc).unwrap();
        assert_eq!(ch.local_balance, 7.0);

        let settled = ch.settle_htlc(PREIMAGE).unwrap();
        assert!((settled - 3.0).abs() < f64::EPSILON);
        assert_eq!(ch.remote_balance, 3.0);
    }

    #[test]
    fn add_htlc_insufficient_balance() {
        let mut ch = AgentLightningChannel::open("A", "B", 1.0, 0.0);
        let htlc = HtlcPayment::new(PREIMAGE, 5.0, 100, "B");
        let err = ch.add_htlc(htlc).unwrap_err();
        assert!(matches!(err, LightningError::InsufficientBalance { .. }));
    }

    #[test]
    fn fail_htlc_refunds_balance() {
        let mut ch = AgentLightningChannel::open("A", "B", 10.0, 0.0);
        let htlc = HtlcPayment::new(PREIMAGE, 4.0, 100, "B");
        ch.add_htlc(htlc).unwrap();
        ch.fail_htlc(PREIMAGE).unwrap();
        assert!((ch.local_balance - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cooperative_close_returns_balances() {
        let mut ch = AgentLightningChannel::open("A", "B", 8.0, 2.0);
        let htlc = HtlcPayment::new(PREIMAGE, 3.0, 100, "B");
        ch.add_htlc(htlc).unwrap();
        ch.settle_htlc(PREIMAGE).unwrap();
        let (local, remote) = ch.cooperative_close();
        assert!((local - 5.0).abs() < f64::EPSILON);
        assert!((remote - 5.0).abs() < f64::EPSILON);
        assert_eq!(ch.state, ChannelState::Closed);
    }

    // ── LightningChannelRouter ───────────────────────────────

    fn three_hop_router() -> (LightningChannelRouter, String, String, String) {
        let mut router = LightningChannelRouter::new();
        let c_ab = router.open_channel("A", "B", 20.0, 0.0).unwrap();
        let c_bc = router.open_channel("B", "C", 20.0, 0.0).unwrap();
        (router, c_ab, c_bc, "C".to_string())
    }

    #[test]
    fn find_route_direct() {
        let (mut router, _, _, _) = three_hop_router();
        router.open_channel("A", "C", 5.0, 5.0).unwrap();
        let route = router.find_route("A", "C", 4.0).unwrap();
        assert_eq!(route.first().unwrap(), "A");
        assert_eq!(route.last().unwrap(), "C");
    }

    #[test]
    fn find_route_multi_hop() {
        let (router, _, _, _) = three_hop_router();
        let route = router.find_route("A", "C", 5.0).unwrap();
        assert_eq!(route, vec!["A", "B", "C"]);
    }

    #[test]
    fn find_route_no_liquidity_fails() {
        let (router, _, _, _) = three_hop_router();
        // Need 100 but channels only have 20
        let err = router.find_route("A", "C", 100.0).unwrap_err();
        assert!(matches!(err, LightningError::NoRoute { .. }));
    }

    #[test]
    fn send_payment_along_route_two_hop() {
        let (mut router, _, _, _) = three_hop_router();
        let route = router.find_route("A", "C", 5.0).unwrap();
        router
            .send_payment_along_route(&route, PREIMAGE, 5.0, 100)
            .unwrap();

        let ch_bc = router
            .get_channel(&AgentLightningChannel::derive_id("B", "C"))
            .unwrap();
        // C's balance on B-C channel should have grown
        assert!(ch_bc.remote_balance > 0.0 || ch_bc.local_balance > 0.0);
    }

    #[test]
    fn duplicate_channel_fails() {
        let mut router = LightningChannelRouter::new();
        router.open_channel("X", "Y", 10.0, 10.0).unwrap();
        let err = router.open_channel("X", "Y", 5.0, 5.0).unwrap_err();
        assert!(matches!(err, LightningError::ChannelAlreadyExists(..)));
    }

    #[test]
    fn close_channel_returns_settlement_record() {
        let mut router = LightningChannelRouter::new();
        let cid = router.open_channel("P", "Q", 10.0, 10.0).unwrap();
        let record = router.close_channel(&cid, 10.0).unwrap();
        assert_eq!(record.local_did, "P");
        assert_eq!(record.remote_did, "Q");
        // No payments, so net delta = 0
        assert!(record.net_delta.abs() < f64::EPSILON);
    }
}
