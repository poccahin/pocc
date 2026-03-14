//! 源路由多跳支付（Sphinx-lite Onion Routing）
//!
//! 比特币闪电网络使用 **源路由**（Source Routing）+ **Sphinx 洋葱包**：
//! - 发送方完整计算路由路径，将其打包为洋葱密文。
//! - 每个中间节点只能解密本层，看不到全貌（隐私保护）。
//! - 本模块实现简化版，不加密洋葱层，但保留路径计算、路由费用和多跳 HTLC 转发语义。
//!
//! ## 路由费用（Routing Fee）
//!
//! 闪电网络每跳向中间节点支付路由费：
//!
//! ```text
//! fee = base_fee_sat + floor(amount_sat × fee_rate_ppm / 1_000_000)
//! ```
//!
//! 因此实际发送额度需从目标金额往源头递推（洋葱包构建方向与支付方向相反）。

use std::collections::{HashMap, VecDeque};
use thiserror::Error;

use crate::channel::BtcLightningChannel;
use crate::invoice::Satoshi;

#[derive(Error, Debug, PartialEq)]
pub enum RouterError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("No route from {src} to {dst}")]
    NoRoute { src: String, dst: String },
    #[error("Channel already exists between {0} and {1}")]
    ChannelAlreadyExists(String, String),
    #[error("Payment failed at hop {hop}: {reason}")]
    HopFailed { hop: usize, reason: String },
}

/// 单条边（通道）的路由参数（BOLT-7 channel_update 字段）
#[derive(Debug, Clone)]
pub struct EdgePolicy {
    /// 基础费（聪），与金额无关
    pub base_fee_sat: Satoshi,
    /// 按比例费率（百万分之一）
    pub fee_rate_ppm: u64,
    /// 该方向的可用流动性（聪）
    pub available_liquidity_sat: Satoshi,
}

impl EdgePolicy {
    /// 计算转发 `amount_sat` 聪的路由费（向上取整）
    pub fn routing_fee(&self, amount_sat: Satoshi) -> Satoshi {
        let proportional = (amount_sat as u128 * self.fee_rate_ppm as u128 / 1_000_000) as u64;
        self.base_fee_sat + proportional
    }
}

/// 多跳路由结果
#[derive(Debug, Clone)]
pub struct RouteHop {
    /// 该跳的节点 ID
    pub node_id: String,
    /// 该跳的通道 ID
    pub channel_id: String,
    /// 该跳需要转发的金额（含后续所有路由费）
    pub amount_sat: Satoshi,
    /// 该跳向中间节点支付的路由费
    pub fee_sat: Satoshi,
    /// 该跳的 HTLC 超时（相对区块）
    pub expiry_blocks: u32,
}

/// 完整路由方案
#[derive(Debug, Clone)]
pub struct Route {
    /// 从发送方到接收方的各跳列表（不含发送方自身）
    pub hops: Vec<RouteHop>,
    /// 总金额（含所有路由费）
    pub total_amount_sat: Satoshi,
    /// 总路由费（所有中间节点费用之和）
    pub total_fee_sat: Satoshi,
}

/// 洋葱支付路由器
///
/// 维护一个节点图（以 channel_id 为边），支持：
/// - `add_channel`: 注册通道并指定双向路由参数
/// - `find_route`: BFS 寻找流动性充足的路径，计算路由费
/// - `execute_payment`: 沿路由逐跳转发 HTLC
pub struct LnRouter {
    /// channel_id → channel
    channels: HashMap<String, BtcLightningChannel>,
    /// 边策略：(channel_id, direction 0/1) → EdgePolicy
    policies: HashMap<(String, u8), EdgePolicy>,
    /// 邻接表：node_id → Vec<(neighbor_id, channel_id)>
    adjacency: HashMap<String, Vec<(String, String)>>,
}

impl Default for LnRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl LnRouter {
    pub fn new() -> Self {
        LnRouter {
            channels: HashMap::new(),
            policies: HashMap::new(),
            adjacency: HashMap::new(),
        }
    }

    /// 注册一条通道并设置双向路由策略
    ///
    /// - `dir0_policy`: local → remote 方向的费用策略
    /// - `dir1_policy`: remote → local 方向的费用策略
    pub fn add_channel(
        &mut self,
        channel: BtcLightningChannel,
        dir0_policy: EdgePolicy,
        dir1_policy: EdgePolicy,
    ) -> Result<(), RouterError> {
        let local = channel.local_node_id.clone();
        let remote = channel.remote_node_id.clone();
        let cid = channel.channel_id.clone();

        if self.channels.contains_key(&cid) {
            return Err(RouterError::ChannelAlreadyExists(local, remote));
        }

        self.adjacency.entry(local.clone()).or_default().push((remote.clone(), cid.clone()));
        self.adjacency.entry(remote.clone()).or_default().push((local.clone(), cid.clone()));

        self.policies.insert((cid.clone(), 0), dir0_policy);
        self.policies.insert((cid.clone(), 1), dir1_policy);
        self.channels.insert(cid, channel);
        Ok(())
    }

    /// 寻找从 `src` 到 `dst` 能转发 `amount_sat` 聪的最短路（跳数最少）
    ///
    /// 返回带路由费的完整 `Route`。
    pub fn find_route(&self, src: &str, dst: &str, amount_sat: Satoshi) -> Result<Route, RouterError> {
        if !self.adjacency.contains_key(src) {
            return Err(RouterError::NodeNotFound(src.to_string()));
        }
        if !self.adjacency.contains_key(dst) {
            return Err(RouterError::NodeNotFound(dst.to_string()));
        }

        // BFS：找最短跳数路径（仅考虑单方向流动性）
        let mut queue: VecDeque<(String, Vec<(String, String)>)> = VecDeque::new();
        let mut visited: HashMap<String, bool> = HashMap::new();

        queue.push_back((src.to_string(), vec![]));
        visited.insert(src.to_string(), true);

        while let Some((current, path)) = queue.pop_front() {
            let neighbors = match self.adjacency.get(&current) {
                Some(n) => n,
                None => continue,
            };

            for (neighbor, cid) in neighbors {
                if visited.contains_key(neighbor) {
                    continue;
                }

                // 确定方向
                let ch = self.channels.get(cid).unwrap();
                let direction = if ch.local_node_id == current { 0u8 } else { 1u8 };
                let policy = &self.policies[&(cid.clone(), direction)];

                // 流动性检查（简化：只检查 available_liquidity_sat）
                if policy.available_liquidity_sat < amount_sat {
                    continue;
                }

                let mut new_path = path.clone();
                new_path.push((neighbor.clone(), cid.clone()));

                if neighbor == dst {
                    // 逆向计算路由费（从目标往源头）
                    return Ok(Self::build_route(src, new_path, amount_sat, &self.policies, &self.channels));
                }

                visited.insert(neighbor.clone(), true);
                queue.push_back((neighbor.clone(), new_path));
            }
        }

        Err(RouterError::NoRoute { src: src.to_string(), dst: dst.to_string() })
    }

    /// 执行支付：沿路由链逐跳添加 HTLC，然后逆向 fulfill
    pub fn execute_payment(
        &mut self,
        route: &Route,
        preimage: &[u8; 32],
    ) -> Result<Satoshi, RouterError> {
        // 正向：逐跳 add_htlc
        for (i, hop) in route.hops.iter().enumerate() {
            let ch = self.channels.get_mut(&hop.channel_id)
                .ok_or_else(|| RouterError::HopFailed { hop: i, reason: format!("channel {} not found", hop.channel_id) })?;

            ch.add_htlc(preimage, hop.amount_sat, hop.expiry_blocks, &hop.node_id)
                .map_err(|e| RouterError::HopFailed { hop: i, reason: e.to_string() })?;
        }

        // 逆向：逐跳 fulfill_htlc（接收方揭示原像，沿路径回传）
        let received_sat = route.hops.last().map(|h| h.amount_sat).unwrap_or(0);
        for (i, hop) in route.hops.iter().enumerate().rev() {
            let ch = self.channels.get_mut(&hop.channel_id)
                .ok_or_else(|| RouterError::HopFailed { hop: i, reason: "channel not found on settle".to_string() })?;

            ch.fulfill_htlc(preimage)
                .map_err(|e| RouterError::HopFailed { hop: i, reason: e.to_string() })?;
        }

        Ok(received_sat)
    }

    /// 读取通道（只读）
    pub fn channel(&self, channel_id: &str) -> Option<&BtcLightningChannel> {
        self.channels.get(channel_id)
    }

    // 从 BFS 找到的路径逆向计算路由费，构建 Route
    fn build_route(
        _src: &str,
        path: Vec<(String, String)>,  // (node_id, channel_id) 不含 src
        target_amount_sat: Satoshi,
        policies: &HashMap<(String, u8), EdgePolicy>,
        channels: &HashMap<String, BtcLightningChannel>,
    ) -> Route {
        // 逆向遍历：每跳需要发送 amount + 后续费用
        let mut hops: Vec<RouteHop> = Vec::with_capacity(path.len());
        let mut running_amount = target_amount_sat;

        for (node_id, cid) in path.iter().rev() {
            let ch = &channels[cid];
            // 找到前驱节点：path 中当前位置的前一个 node，或 src
            let prev_is_local = ch.local_node_id != *node_id;
            let direction = if prev_is_local { 0u8 } else { 1u8 };
            let policy = &policies[&(cid.clone(), direction)];
            let fee = if hops.is_empty() { 0 } else { policy.routing_fee(running_amount) };

            hops.push(RouteHop {
                node_id: node_id.clone(),
                channel_id: cid.clone(),
                amount_sat: running_amount,
                fee_sat: fee,
                expiry_blocks: 144,
            });
            running_amount += fee;
        }

        hops.reverse();
        let total_fee_sat: u64 = hops.iter().map(|h| h.fee_sat).sum();
        let total_amount_sat = hops.first().map(|h| h.amount_sat).unwrap_or(target_amount_sat);

        Route { hops, total_amount_sat, total_fee_sat }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::BtcLightningChannel;

    const PREIMAGE: [u8; 32] = [0xAB; 32];

    fn default_policy(liquidity: Satoshi) -> EdgePolicy {
        EdgePolicy { base_fee_sat: 1, fee_rate_ppm: 100, available_liquidity_sat: liquidity }
    }

    fn two_hop_router() -> LnRouter {
        let mut router = LnRouter::new();
        let ch_ab = BtcLightningChannel::open("alice", "bob", 500_000, 500_000);
        let ch_bc = BtcLightningChannel::open("bob", "carol", 500_000, 500_000);
        router.add_channel(ch_ab, default_policy(500_000), default_policy(500_000)).unwrap();
        router.add_channel(ch_bc, default_policy(500_000), default_policy(500_000)).unwrap();
        router
    }

    #[test]
    fn find_direct_route() {
        let router = two_hop_router();
        let route = router.find_route("alice", "bob", 10_000).unwrap();
        assert_eq!(route.hops.len(), 1);
        assert_eq!(route.hops[0].node_id, "bob");
    }

    #[test]
    fn find_two_hop_route() {
        let router = two_hop_router();
        let route = router.find_route("alice", "carol", 10_000).unwrap();
        assert_eq!(route.hops.len(), 2);
        assert_eq!(route.hops.last().unwrap().node_id, "carol");
    }

    #[test]
    fn no_route_for_insufficient_liquidity() {
        let router = two_hop_router();
        let err = router.find_route("alice", "carol", 600_000).unwrap_err();
        assert!(matches!(err, RouterError::NoRoute { .. }));
    }

    #[test]
    fn execute_payment_two_hop() {
        let mut router = two_hop_router();
        let route = router.find_route("alice", "carol", 10_000).unwrap();
        let received = router.execute_payment(&route, &PREIMAGE).unwrap();
        assert_eq!(received, 10_000);

        // carol's channel should have gained 10_000
        let cid_bc = BtcLightningChannel::derive_channel_id("bob", "carol");
        let ch = router.channel(&cid_bc).unwrap();
        assert_eq!(ch.remote_balance_sat, 510_000);
    }

    #[test]
    fn routing_fee_calculation() {
        let policy = EdgePolicy { base_fee_sat: 10, fee_rate_ppm: 1_000, available_liquidity_sat: 1_000_000 };
        // fee = 10 + floor(100_000 * 1000 / 1_000_000) = 10 + 100 = 110
        assert_eq!(policy.routing_fee(100_000), 110);
    }
}
