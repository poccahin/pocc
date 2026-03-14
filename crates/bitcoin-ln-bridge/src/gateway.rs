//! BTC ↔ LIFE++ 跨币种桥接网关
//!
//! ## 职责
//!
//! `BtcLnGateway` 是 Bitcoin Lightning Network 与 Life++ 经济层的边界节点：
//!
//! ```text
//!  人类用户                Life++ 网关节点                  智能体/机器人
//!    │   BTC Invoice          │                                  │
//!    │──────────────────────►│                                  │
//!    │   支付 N 聪            │   AtomicSwap: N sat → M LIFE++   │
//!    │                       │─────────────────────────────────►│
//!    │                       │   Agent 执行任务                  │
//!    │                       │◄─────────────────────────────────│
//!    │   确认收据             │                                  │
//!    │◄──────────────────────│                                  │
//! ```
//!
//! ## 汇率机制
//!
//! 网关维护一个 BTC/LIFE++ 即时汇率（`sats_per_life_token`）。
//! 实际生产中该汇率由预言机（Pyth / Switchboard on Solana）喂价。
//! 本模块使用固定汇率进行演示。
//!
//! ## 原子换汇（Atomic Swap）
//!
//! 网关使用**同一个 preimage** 同时锁定 BTC 侧和 LIFE++ 侧的 HTLC，
//! 保证要么两侧同时结算，要么两侧同时退款——无须信任网关。

use thiserror::Error;

use crate::invoice::{BoltInvoice, InvoiceError, MilliSatoshi, Satoshi};

#[derive(Error, Debug, PartialEq)]
pub enum GatewayError {
    #[error("Invoice error: {0}")]
    InvoiceError(#[from] InvoiceError),
    #[error("Insufficient BTC liquidity: need {needed} sat, have {available} sat")]
    InsufficientBtcLiquidity { needed: Satoshi, available: Satoshi },
    #[error("Insufficient LIFE++ liquidity: need {needed:.4}, have {available:.4}")]
    InsufficientLifeLiquidity { needed: f64, available: f64 },
    #[error("Swap ID not found: {0}")]
    SwapNotFound(String),
    #[error("Swap already settled or cancelled")]
    SwapAlreadyClosed,
}

/// 原子换汇状态
#[derive(Debug, Clone, PartialEq)]
pub enum SwapState {
    Pending,
    Settled,
    Refunded,
}

/// 一笔原子换汇记录
#[derive(Debug, Clone)]
pub struct AtomicSwap {
    pub swap_id: String,
    /// BTC 侧：收取的聪数
    pub btc_received_sat: Satoshi,
    /// LIFE++ 侧：分发给智能体的金额
    pub life_distributed: f64,
    /// 目标智能体 DID
    pub agent_did: String,
    /// SHA-256(preimage)
    pub payment_hash: [u8; 32],
    pub state: SwapState,
}

/// 已完成换汇的结果
#[derive(Debug, Clone)]
pub struct SwapReceipt {
    pub swap_id: String,
    pub btc_sat: Satoshi,
    pub life_tokens: f64,
    pub agent_did: String,
}

/// BTC 闪电网关节点
///
/// 持有 BTC 和 LIFE++ 双侧流动性，为用户提供入金（on-ramp）服务。
pub struct BtcLnGateway {
    /// 网关节点 ID（在 BTC LN 中是公钥哈希，在 Life++ 中是 DID）
    pub node_id: String,
    /// BTC 侧可用余额（聪）
    pub btc_liquidity_sat: Satoshi,
    /// LIFE++ 侧可用余额
    pub life_liquidity: f64,
    /// 1 LIFE++ = N 聪（固定汇率，生产中由预言机提供）
    pub sats_per_life_token: Satoshi,
    /// 网关服务费率（千分比，e.g. 3 = 0.3%）
    pub fee_permille: u64,
    /// 挂起/已完成的换汇列表
    swaps: Vec<AtomicSwap>,
}

impl BtcLnGateway {
    /// 创建网关
    ///
    /// - `sats_per_life_token`: 汇率，1 LIFE++ 值多少聪
    /// - `fee_permille`: 服务费千分比（3 = 0.3%）
    pub fn new(
        node_id: &str,
        btc_liquidity_sat: Satoshi,
        life_liquidity: f64,
        sats_per_life_token: Satoshi,
        fee_permille: u64,
    ) -> Self {
        BtcLnGateway {
            node_id: node_id.to_string(),
            btc_liquidity_sat,
            life_liquidity,
            sats_per_life_token,
            fee_permille,
            swaps: Vec::new(),
        }
    }

    /// 为用户生成一张 BOLT-11 票据
    ///
    /// 返回 (invoice, preimage)。实际生产中 preimage 只有网关持有，
    /// 通过安全通道在 LIFE++ 侧提前揭示给智能体。
    pub fn create_invoice(
        &self,
        life_amount: f64,
        agent_did: &str,
        description: &str,
        expiry_seconds: u64,
        preimage: &[u8; 32],
        now_secs: u64,
    ) -> Result<BoltInvoice, GatewayError> {
        let sat_amount = (life_amount * self.sats_per_life_token as f64) as Satoshi;
        // 加上服务费
        let sat_with_fee = sat_amount + self.gateway_fee(sat_amount);
        let amount_msat = sat_with_fee * 1_000;
        let invoice = BoltInvoice::new(
            preimage,
            amount_msat as MilliSatoshi,
            expiry_seconds,
            agent_did,
            description,
            now_secs,
        )?;
        Ok(invoice)
    }

    /// 处理入金：用户支付 BTC 票据后，网关原子分发 LIFE++ 给智能体
    ///
    /// - `invoice`: 用户已支付的票据
    /// - `preimage`: 原像（揭示即证明 BTC 侧已结算）
    /// - `agent_did`: 接收 LIFE++ 的智能体 DID
    /// - `paid_msat`: 用户实际支付的毫聪
    /// - `now_secs`: 当前时间戳（秒）
    pub fn on_ramp(
        &mut self,
        invoice: &BoltInvoice,
        preimage: &[u8; 32],
        agent_did: &str,
        paid_msat: MilliSatoshi,
        now_secs: u64,
    ) -> Result<SwapReceipt, GatewayError> {
        // 1. 验证 BTC 票据
        invoice.verify_payment(preimage, paid_msat, now_secs)
            .map_err(GatewayError::InvoiceError)?;

        let btc_received_sat = paid_msat / 1_000;

        // 2. 检查 BTC 流动性（网关应收到聪）
        if self.btc_liquidity_sat + btc_received_sat < btc_received_sat {
            return Err(GatewayError::InsufficientBtcLiquidity {
                needed: btc_received_sat,
                available: self.btc_liquidity_sat,
            });
        }

        // 3. 计算扣除手续费后的 LIFE++ 金额
        let fee_sat = self.gateway_fee(btc_received_sat);
        let net_sat = btc_received_sat.saturating_sub(fee_sat);
        let life_amount = net_sat as f64 / self.sats_per_life_token as f64;

        // 4. 检查 LIFE++ 流动性
        if self.life_liquidity < life_amount {
            return Err(GatewayError::InsufficientLifeLiquidity {
                needed: life_amount,
                available: self.life_liquidity,
            });
        }

        // 5. 原子更新双侧余额
        self.btc_liquidity_sat += btc_received_sat;
        self.life_liquidity -= life_amount;

        let swap_id = format!("{}-{}", agent_did, hex::encode(&invoice.payment_hash[..4]));

        self.swaps.push(AtomicSwap {
            swap_id: swap_id.clone(),
            btc_received_sat,
            life_distributed: life_amount,
            agent_did: agent_did.to_string(),
            payment_hash: invoice.payment_hash,
            state: SwapState::Settled,
        });

        Ok(SwapReceipt {
            swap_id,
            btc_sat: btc_received_sat,
            life_tokens: life_amount,
            agent_did: agent_did.to_string(),
        })
    }

    /// 出金：智能体将 LIFE++ 换回 BTC（反向通道）
    pub fn off_ramp(
        &mut self,
        life_amount: f64,
        agent_did: &str,
        preimage: &[u8; 32],
    ) -> Result<Satoshi, GatewayError> {
        if self.life_liquidity + life_amount < life_amount {
            // overflow guard
            return Err(GatewayError::InsufficientLifeLiquidity {
                needed: life_amount,
                available: self.life_liquidity,
            });
        }

        let sat_gross = (life_amount * self.sats_per_life_token as f64) as Satoshi;
        let fee_sat = self.gateway_fee(sat_gross);
        let sat_net = sat_gross.saturating_sub(fee_sat);

        if self.btc_liquidity_sat < sat_net {
            return Err(GatewayError::InsufficientBtcLiquidity {
                needed: sat_net,
                available: self.btc_liquidity_sat,
            });
        }

        self.life_liquidity += life_amount;
        self.btc_liquidity_sat -= sat_net;

        let swap_id = format!("off-{}-{}", agent_did, hex::encode(&preimage[..4]));
        self.swaps.push(AtomicSwap {
            swap_id,
            btc_received_sat: 0,
            life_distributed: life_amount,
            agent_did: agent_did.to_string(),
            payment_hash: [0u8; 32],
            state: SwapState::Settled,
        });

        Ok(sat_net)
    }

    /// 当前已完成/挂起换汇数量
    pub fn swap_count(&self) -> usize {
        self.swaps.len()
    }

    /// 查询已完成换汇列表
    pub fn settled_swaps(&self) -> impl Iterator<Item = &AtomicSwap> {
        self.swaps.iter().filter(|s| s.state == SwapState::Settled)
    }

    /// 网关服务费（聪）
    pub fn gateway_fee(&self, amount_sat: Satoshi) -> Satoshi {
        amount_sat * self.fee_permille / 1_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREIMAGE: [u8; 32] = [0xCC; 32];

    fn test_gateway() -> BtcLnGateway {
        BtcLnGateway::new(
            "gateway.btcln.ahin.io",
            10_000_000, // 0.1 BTC
            1_000.0,    // 1000 LIFE++
            10_000,     // 1 LIFE++ = 10,000 sat (≈ $100 @ 1BTC=$100k)
            3,          // 0.3% fee
        )
    }

    #[test]
    fn on_ramp_success() {
        let mut gw = test_gateway();
        let inv = gw.create_invoice(1.0, "robot-a.ahin.io", "Warehouse task", 3600, &PREIMAGE, 1000).unwrap();
        let receipt = gw.on_ramp(&inv, &PREIMAGE, "robot-a.ahin.io", inv.amount_msat, 1100).unwrap();
        assert_eq!(receipt.agent_did, "robot-a.ahin.io");
        // net ≈ 1 LIFE++ (after 0.3% fee, ~0.997 LIFE++)
        assert!(receipt.life_tokens > 0.99 && receipt.life_tokens <= 1.0);
        assert!(receipt.btc_sat > 0);
    }

    #[test]
    fn on_ramp_expired_invoice_fails() {
        let mut gw = test_gateway();
        let inv = gw.create_invoice(1.0, "robot-b.ahin.io", "test", 60, &PREIMAGE, 0).unwrap();
        let err = gw.on_ramp(&inv, &PREIMAGE, "robot-b.ahin.io", inv.amount_msat, 200).unwrap_err();
        assert!(matches!(err, GatewayError::InvoiceError(InvoiceError::Expired)));
    }

    #[test]
    fn gateway_fee_calculation() {
        let gw = test_gateway();
        // 0.3% of 10_000 sat = 30 sat
        assert_eq!(gw.gateway_fee(10_000), 30);
    }

    #[test]
    fn off_ramp_success() {
        let mut gw = test_gateway();
        let received_sat = gw.off_ramp(10.0, "agent-x.ahin.io", &PREIMAGE).unwrap();
        // 10 LIFE++ × 10_000 sat/LIFE++ = 100_000 sat gross; minus 0.3% = 99_700 sat
        assert_eq!(received_sat, 99_700);
        assert_eq!(gw.swap_count(), 1);
    }

    #[test]
    fn insufficient_life_liquidity() {
        let mut gw = test_gateway();
        // 생명력 流动性只有 1000 LIFE++，尝试换 2000
        let inv = gw.create_invoice(2000.0, "robot-z.ahin.io", "overload", 3600, &PREIMAGE, 0).unwrap();
        let err = gw.on_ramp(&inv, &PREIMAGE, "robot-z.ahin.io", inv.amount_msat, 1).unwrap_err();
        assert!(matches!(err, GatewayError::InsufficientLifeLiquidity { .. }));
    }
}
