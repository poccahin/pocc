//! BOLT-11 简化票据（Invoice）
//!
//! 比特币闪电网络使用 BOLT-11 格式的票据进行支付。本模块实现其核心语义子集：
//! - **payment_hash**: SHA-256(payment_preimage)，用于 HTLC 哈希锁
//! - **amount_msat**: 支付金额（毫聪，millisatoshi，1 BTC = 100_000_000 sat = 100_000_000_000 msat）
//! - **expiry_seconds**: 票据有效期（默认 3600 秒 = 1 小时）
//! - **description**: 人类可读的支付描述（对应 BOLT-11 `d` 字段）
//! - **payee_pubkey**: 收款方公钥哈希（这里用 DID 字符串模拟）

use sha2::{Digest, Sha256};
use thiserror::Error;

/// 毫聪（millisatoshi）— 闪电网络的最小计量单位
pub type MilliSatoshi = u64;
/// 聪（satoshi）— 比特币链上最小单位
pub type Satoshi = u64;

/// 1 BTC = 100_000_000 聪
pub const SATS_PER_BTC: Satoshi = 100_000_000;
/// 1 聪 = 1_000 毫聪
pub const MSAT_PER_SAT: MilliSatoshi = 1_000;

#[derive(Error, Debug, PartialEq)]
pub enum InvoiceError {
    #[error("Invoice expired")]
    Expired,
    #[error("Amount mismatch: invoice expects {expected} msat, got {actual} msat")]
    AmountMismatch { expected: MilliSatoshi, actual: MilliSatoshi },
    #[error("Invalid preimage: does not match payment_hash")]
    InvalidPreimage,
    #[error("Amount must be greater than zero")]
    ZeroAmount,
}

/// BOLT-11 简化票据
#[derive(Debug, Clone)]
pub struct BoltInvoice {
    /// SHA-256(payment_preimage)
    pub payment_hash: [u8; 32],
    /// 支付金额（毫聪）
    pub amount_msat: MilliSatoshi,
    /// 票据有效期（秒）
    pub expiry_seconds: u64,
    /// 收款方标识（BTC 公钥哈希或 Life++ DID）
    pub payee_id: String,
    /// 人类可读描述（BOLT-11 `d` 字段）
    pub description: String,
    /// 创建时间戳（Unix 秒）
    pub created_at: u64,
}

impl BoltInvoice {
    /// 从支付原像（preimage）创建票据
    ///
    /// # 参数
    /// - `preimage` — 随机 32 字节秘密；只有发票人知道，支付完成后才公开
    /// - `amount_msat` — 请求金额（毫聪）
    /// - `expiry_seconds` — 票据有效秒数（通常 3600）
    /// - `payee_id` — 收款方节点 ID 或 Life++ DID
    /// - `description` — 本次支付的描述字符串
    /// - `created_at` — 创建时刻的 Unix 时间戳（秒）
    pub fn new(
        preimage: &[u8; 32],
        amount_msat: MilliSatoshi,
        expiry_seconds: u64,
        payee_id: &str,
        description: &str,
        created_at: u64,
    ) -> Result<Self, InvoiceError> {
        if amount_msat == 0 {
            return Err(InvoiceError::ZeroAmount);
        }
        let payment_hash = Self::hash_preimage(preimage);
        Ok(BoltInvoice {
            payment_hash,
            amount_msat,
            expiry_seconds,
            payee_id: payee_id.to_string(),
            description: description.to_string(),
            created_at,
        })
    }

    /// 检查票据在给定时间戳（秒）是否仍有效
    pub fn is_valid_at(&self, now_secs: u64) -> bool {
        now_secs < self.created_at.saturating_add(self.expiry_seconds)
    }

    /// 验证支付方提供的原像和金额是否正确
    pub fn verify_payment(
        &self,
        preimage: &[u8; 32],
        paid_msat: MilliSatoshi,
        now_secs: u64,
    ) -> Result<(), InvoiceError> {
        if !self.is_valid_at(now_secs) {
            return Err(InvoiceError::Expired);
        }
        if Self::hash_preimage(preimage) != self.payment_hash {
            return Err(InvoiceError::InvalidPreimage);
        }
        if paid_msat < self.amount_msat {
            return Err(InvoiceError::AmountMismatch {
                expected: self.amount_msat,
                actual: paid_msat,
            });
        }
        Ok(())
    }

    /// 将金额转换为聪（截断毫聪余量）
    pub fn amount_sats(&self) -> Satoshi {
        self.amount_msat / MSAT_PER_SAT
    }

    fn hash_preimage(preimage: &[u8; 32]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(preimage);
        let mut out = [0u8; 32];
        out.copy_from_slice(&h.finalize());
        out
    }
}

/// 生成一个人类可读的票据摘要字符串（模拟 `lnbc...` 编码）
pub fn invoice_summary(inv: &BoltInvoice) -> String {
    let hash_prefix = hex::encode(&inv.payment_hash[..4]);
    format!(
        "lnbc{}msat 1p{} | {} | expires in {}s",
        inv.amount_msat,
        hash_prefix,
        inv.description,
        inv.expiry_seconds,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREIMAGE: [u8; 32] = [0xde, 0xad, 0xbe, 0xef, 0, 0, 0, 0,
                                  0, 0, 0, 0, 0, 0, 0, 0,
                                  0, 0, 0, 0, 0, 0, 0, 0,
                                  0, 0, 0, 0, 0, 0, 0, 0];

    #[test]
    fn create_and_verify_invoice() {
        let inv = BoltInvoice::new(&PREIMAGE, 1_000_000, 3600, "agent-b.ahin.io",
                                    "Warehouse cleaning task", 1_000_000).unwrap();
        assert_eq!(inv.amount_sats(), 1_000);
        assert!(inv.is_valid_at(1_001_000));
        assert!(!inv.is_valid_at(1_004_001));
        inv.verify_payment(&PREIMAGE, 1_000_000, 1_001_000).unwrap();
    }

    #[test]
    fn expired_invoice_rejected() {
        let inv = BoltInvoice::new(&PREIMAGE, 500, 100, "agent-c.ahin.io",
                                    "data query", 1_000).unwrap();
        let err = inv.verify_payment(&PREIMAGE, 500, 2_000).unwrap_err();
        assert_eq!(err, InvoiceError::Expired);
    }

    #[test]
    fn wrong_preimage_rejected() {
        let inv = BoltInvoice::new(&PREIMAGE, 1000, 3600, "x.ahin.io", "test", 0).unwrap();
        let bad: [u8; 32] = [0xFF; 32];
        assert_eq!(inv.verify_payment(&bad, 1000, 0).unwrap_err(), InvoiceError::InvalidPreimage);
    }

    #[test]
    fn underpayment_rejected() {
        let inv = BoltInvoice::new(&PREIMAGE, 1000, 3600, "x.ahin.io", "test", 0).unwrap();
        let err = inv.verify_payment(&PREIMAGE, 999, 0).unwrap_err();
        assert!(matches!(err, InvoiceError::AmountMismatch { .. }));
    }

    #[test]
    fn zero_amount_rejected() {
        assert_eq!(BoltInvoice::new(&PREIMAGE, 0, 3600, "x", "t", 0).unwrap_err(), InvoiceError::ZeroAmount);
    }
}
