//! Bitcoin Lightning Network Bridge for Life++
//!
//! 本 crate 对比特币闪电网络（Bitcoin Lightning Network, BLN）进行适用性评估，
//! 并提供可直接运行的桥接框架，将 BTC 闪电支付接入 Life++ 智能体经济层。
//!
//! # 模块概览
//!
//! | 模块 | 描述 |
//! |---|---|
//! | [`invoice`] | BOLT-11 简化票据（payment_hash, amount_msat, expiry） |
//! | [`channel`] | BTC 计价双向支付通道（satoshi 余额、HTLC 生命周期） |
//! | [`router`]  | 源路由洋葱支付（Sphinx-lite 多跳 HTLC 转发） |
//! | [`gateway`] | BTC ↔ LIFE++ 跨币种桥接网关 |

pub mod channel;
pub mod gateway;
pub mod invoice;
pub mod router;
