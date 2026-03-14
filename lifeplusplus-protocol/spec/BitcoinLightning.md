# Bitcoin Lightning Network × Life++ 适用性评估与框架规范

**Bitcoin Lightning Network (BLN) Evaluation, Applicability Assessment & Integration Framework**

---

## 1. 摘要 (Abstract)

Bitcoin Lightning Network（BLN）是迄今为止规模最大、运行时间最长的支付通道网络（7+ 年，>15,000 公共节点，>5,000 BTC 通道容量）。本规范对其在 Life++ 智能体协作经济中的适用性进行全面评估，给出三层分层集成框架，并提供完全可运行的代码实现。

> **结论**：BLN 不应取代 Life++ 内部的 x402 通道，而应作为**人类用户 BTC 入金通道**和**全球机构级结算层**，通过 `BtcLnGateway` 桥接到 LIFE++ 经济体系。

---

## 2. Bitcoin Lightning Network 核心机制

### 2.1 BOLT 规范体系

| BOLT | 名称 | 核心内容 |
|---|---|---|
| BOLT-1 | Base Protocol | P2P 消息层，`init`/`ping`/`pong` |
| BOLT-2 | Peer Protocol | 通道开关、承诺交易、HTLC 生命周期 |
| BOLT-4 | Onion Routing | Sphinx 洋葱包，源路由隐私保护 |
| BOLT-7 | P2P Node & Channel Discovery | Gossip 协议，`channel_announcement`/`channel_update` |
| BOLT-11 | Invoice Protocol | 编码发票格式（`lnbc...`） |
| BOLT-12 | Offers | 可重用报价（新一代发票，取代 BOLT-11） |

### 2.2 支付通道状态机（BOLT-2）

```text
  [链上 funding_tx] ─────────────────────────────────────────────────────────
         │
         ▼
    ChannelState::Open
         │
         │  update_add_htlc       ─► HTLC 挂起（锁定余额）
         │  update_fulfill_htlc   ─► 原像揭示，远端余额增加
         │  update_fail_htlc      ─► 超时/失败，退款发送方
         │  commitment_signed     ─► 双方签署新承诺交易
         │
         ▼
    ChannelState::Closing (cooperative: shutdown + closing_signed)
         │
         ▼
    [链上 closing_tx 广播] ─────────────────────────────────────────────────
```

### 2.3 路由费用模型（BOLT-7）

```
routing_fee = base_fee_msat + floor(amount_msat × fee_rate_ppm / 1_000_000)
```

中间节点通过收取路由费获得激励，这是 BLN 中"被动"路由节点的盈利模式。

---

## 3. 适用性评估

### 3.1 优势（✅ 推荐用于）

| 维度 | 评估 |
|---|---|
| **信任基础** | BTC 是全球最去中心化的价值存储，无对手方风险 |
| **协议成熟度** | BOLT 规范运行 7+ 年，安全漏洞已充分暴露和修复 |
| **支付时延** | 通道内支付 < 500ms，远优于链上 10 分钟确认 |
| **全球流动性** | 公共节点 > 15,000，任意两点最多 3-4 跳可达 |
| **原子性** | HTLC 保证要么全额结算，要么全额退款 |
| **隐私性** | Sphinx 洋葱路由，中间节点仅知道前后两跳 |
| **跨链互换** | 同一 preimage 可同时锁定 BTC 和 LIFE++ HTLC，实现无信任换汇 |

### 3.2 局限性（❌ 不推荐用于）

| 局限 | 说明 | Life++ 替代方案 |
|---|---|---|
| 通道开关慢 | 链上 funding/closing 需 1-6 个 10 分钟区块确认 | Solana 400ms 最终性 |
| Bitcoin 脚本受限 | 无法执行 PoCC 联名质押、PoTE 废热证明等复杂逻辑 | Solana Anchor 合约 |
| 无智能合约原语 | 不支持条件结算、声誉惩没、二次方质押等机制 | Solana / Quorum |
| 流动性管理复杂 | 通道单向耗尽后须再平衡（Submarine Swap / Loop Out） | x402 双向通道自动平衡 |
| 单位精度限制 | 最小 1 毫聪（msat），无法处理纳秒级算力计费 | x402 内存级微结算 |

### 3.3 场景适用性矩阵

| 场景 | 适用等级 | 说明 |
|---|---|---|
| 人类用户 BTC 入金（On-Ramp） | ✅ **强烈推荐** | 用户用 BTC LN 支付，网关换成 LIFE++ |
| 跨国/跨机构大额结算（>0.1 BTC） | ✅ **适用** | BTC 作为全球储备货币结算 |
| Agent-to-Agent 纳秒级微支付 | ❌ **不适用** | 改用 x402 AgentLightningChannel |
| PoCC 联名质押与惩没 | ❌ **不适用** | 需要 Solana 智能合约逻辑 |
| 机器人做功热力学报酬 | 🔶 **部分适用** | 毫聪精度可接受，但通道费用较高 |
| RWA 债券结算（需合规 KYC） | 🔶 **部分适用** | 需搭配 LSP + KYC 合规层 |
| 多 Agent 协作日终净额结算 | ✅ **适用** | 通过 BtcLnGateway 和 DailyNettingProcessor |

---

## 4. 三层分层框架

```
┌─────────────────────────────────────────────────────────────────────────┐
│  L1: Bitcoin Lightning Network（入金层 / 全球结算层）                    │
│                                                                         │
│  Human ──[BTC Invoice]──► BtcLnGateway ──[AtomicSwap]──► LIFE++ Pool   │
│                                                                         │
│  • BoltInvoice (BOLT-11): 用户扫码支付                                  │
│  • BtcLightningChannel: 网关在 BTC LN 中持有的通道                       │
│  • LnRouter (source routing): 多跳 HTLC 路径计算                        │
│  • BtcLnGateway: 原子换汇，同一 preimage 锁定双侧 HTLC                  │
└────────────────────────────────┬────────────────────────────────────────┘
                                  │ 换汇: N sat → M LIFE++
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  L2: Life++ x402 闪电通道（智能体协作支付层）                            │
│                                                                         │
│  Agent-A ══[AgentLightningChannel]══ Agent-B ══[...]══ Agent-N          │
│                                                                         │
│  • AgentLightningChannel: 双向 LIFE++ 余额，nonce 防重放                 │
│  • HtlcPayment (SHA-256 哈希锁): 跨 Agent 原子结算                       │
│  • LightningChannelRouter: BFS 流动性感知多跳路由                        │
│  • 每日轧账: DailyNettingProcessor → SettlementBatch                    │
└────────────────────────────────┬────────────────────────────────────────┘
                                  │ 净头寸批量上链
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  L3: Solana / Quorum 链上结算（最终锚定层）                              │
│                                                                         │
│  SettlementBatch.batch_hash ──► Anchor 合约 / Quorum 暗池               │
│                                                                         │
│  • ZK-Compressor: 将批次压缩为单个 SNARK 证明                           │
│  • 全局状态根 (globalStateRoot) 更新                                     │
│  • PoCC/PoTE 惩没逻辑在链上执行                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 5. BtcLnGateway 原子换汇协议

### 5.1 入金流程（BTC → LIFE++）

```
1. Agent 向网关申请任务报酬发票（create_invoice）
   • 网关生成随机 preimage，计算 payment_hash
   • 金额 = life_amount × sats_per_life_token × (1 + fee_permille/1000)

2. 用户通过 BTC 闪电钱包支付发票
   • BTC LN 网络完成多跳 HTLC 路由
   • 最终节点（网关）揭示 preimage，BTC 结算

3. 网关原子分发 LIFE++（on_ramp）
   • 验证 preimage 与 payment_hash 匹配
   • 扣除手续费后，从 LIFE++ 流动性池向 Agent DID 转账
   • 同一 preimage 保证：BTC 结算 ⟺ LIFE++ 分发（原子性）

4. 记录 AtomicSwap，可审计
```

### 5.2 出金流程（LIFE++ → BTC）

```
1. Agent 申请兑换 LIFE++ 为 BTC
2. 网关生成 BTC 发票，Agent 扣除 LIFE++ 余额
3. 网关通过 BTC LN 向 Agent 的 BTC 地址付款
4. 同一 preimage 保证：LIFE++ 扣除 ⟺ BTC 到账
```

---

## 6. 实现位置

| 模块 | 路径 | 职责 |
|---|---|---|
| `BoltInvoice` | `crates/bitcoin-ln-bridge/src/invoice.rs` | BOLT-11 票据创建、验证、过期检查 |
| `BtcLightningChannel` | `crates/bitcoin-ln-bridge/src/channel.rs` | BTC 计价双向通道，HTLC 全生命周期 |
| `LnRouter` | `crates/bitcoin-ln-bridge/src/router.rs` | 源路由路径发现，路由费计算，多跳 HTLC 执行 |
| `BtcLnGateway` | `crates/bitcoin-ln-bridge/src/gateway.rs` | 原子换汇网关，入金/出金，流动性管理 |
| `demo` | `crates/bitcoin-ln-bridge/src/bin/demo.rs` | 端到端可运行演示（4 个场景） |

---

## 7. 快速开始（全量可运行代码）

```bash
# 在仓库根目录
cargo run --bin demo -p bitcoin-ln-bridge

# 运行所有单元测试（22 个测试用例）
cargo test -p bitcoin-ln-bridge
```

### 7.1 创建并支付 BTC 发票

```rust
use bitcoin_ln_bridge::{gateway::BtcLnGateway, invoice::invoice_summary};

let preimage: [u8; 32] = rand::random();
let mut gateway = BtcLnGateway::new("gateway.btcln.ahin.io", 50_000_000, 10_000.0, 10_000, 3);

// 为智能体任务创建发票
let invoice = gateway.create_invoice(5.0, "robot-001.ahin.io", "Warehouse task", 3600, &preimage, now)?;
println!("{}", invoice_summary(&invoice));

// 用户支付后，网关换汇
let receipt = gateway.on_ramp(&invoice, &preimage, "robot-001.ahin.io", invoice.amount_msat, now + 60)?;
println!("Agent received: {:.4} LIFE++", receipt.life_tokens);
```

### 7.2 多跳路由支付

```rust
use bitcoin_ln_bridge::{channel::BtcLightningChannel, router::{EdgePolicy, LnRouter}};

let mut router = LnRouter::new();
router.add_channel(BtcLightningChannel::open("alice", "bob", 1_000_000, 0),
    EdgePolicy { base_fee_sat: 1, fee_rate_ppm: 100, available_liquidity_sat: 1_000_000 },
    EdgePolicy { base_fee_sat: 1, fee_rate_ppm: 100, available_liquidity_sat: 0 },
)?;

let route = router.find_route("alice", "bob", 50_000)?;
let received = router.execute_payment(&route, &preimage)?;
```

---

## 8. 与 GOAT 闪电网络规范的对比

| 维度 | Bitcoin LN（本规范） | GOAT/x402 Life++ LN |
|---|---|---|
| **结算货币** | 聪（satoshi）/ BTC | LIFE++ 代币 |
| **通道精度** | 毫聪（1 msat = 10⁻¹¹ BTC） | 浮点 LIFE++（内存级） |
| **路由算法** | 源路由 + Sphinx 洋葱包 | BFS 流动性感知（无洋葱） |
| **密码学** | secp256k1（未来 Taproot Musig2） | Falcon-1024 + Kyber-1024（PQC） |
| **链上结算** | Bitcoin（10 分钟确认） | Solana（400ms 最终性） |
| **智能合约** | ❌ Bitcoin Script 受限 | ✅ Solana Anchor |
| **主要用途** | 人类入金、全球结算 | Agent-to-Agent 协作支付 |
| **推荐集成** | 通过 BtcLnGateway 桥接 L2 | 原生 Life++ 经济层 |

---

## 9. 生产部署建议

1. **选择 BTC LN 节点实现**: [LDK (Rust)](https://lightningdevkit.org/) 推荐集成到 openclaw-runtime；[LND (Go)](https://github.com/lightningnetwork/lnd) 适用于独立网关服务。

2. **流动性管理**: 网关需维持 BTC（LN）和 LIFE++（Solana）双侧流动性池。使用 [Loop Out](https://lightning.engineering/loop/) 进行通道再平衡。

3. **汇率预言机**: 接入 [Pyth Network](https://pyth.network/) 或 [Switchboard](https://switchboard.xyz/) 获取实时 BTC/LIFE++ 价格，取代 `BtcLnGateway` 中的固定汇率。

4. **安全审计**: HTLC 超时参数（`expiry_blocks`）应根据路由跳数递减（类 CLTV delta），防止时间错位攻击（Timelock Attack）。

5. **LSP 集成**: 使用 [Lightning Service Provider (LSP)](https://github.com/BitcoinAndLightningLayerSpecs/lsp) 规范为新 Agent 节点提供入站流动性（Inbound Liquidity），解决"冷启动"问题。
