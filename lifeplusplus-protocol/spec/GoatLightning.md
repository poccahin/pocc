# GOAT 闪电网络 × Life++ 智能体协作支付规范

**GOAT Lightning Network Application for Life++ Inter-Agent Payment, Clearing & Settlement**

---

## 1. 摘要 (Abstract)

[GOAT-Hackathon-2026](https://github.com/eigmax/GOAT-Hackathon-2026) 提出将 AI Agent 作为闪电网络（Lightning Network）中的主动路由节点，实现"智能体驱动的链下微支付"。本规范将这一思想映射到 Life++ 协议栈：

> **Life++ 中的每一个 AI Agent / 物理机器人都同时是闪电通道的端点与路由节点。**

智能体协作所产生的微观支付（每一次推理调用、每一次物理做功）通过 **x402 状态通道（AgentLightningChannel）** 在链下高频结算；日终净额轧账通过 **DailyNettingProcessor** 压缩为多边净头寸；最终仅将净差额批量提交至 Solana/Quorum 结算层，实现"百万次微支付 → 一笔链上交易"的极致效率。

---

## 2. 核心概念映射

| GOAT 闪电网络概念 | Life++ 对应实现 |
|---|---|
| 支付通道（Payment Channel） | `AgentLightningChannel`（双向余额账本） |
| HTLC（哈希时间锁合约） | `HtlcPayment`（SHA-256 哈希锁 + 区块超时） |
| 多跳路由（Multi-hop Routing） | `LightningChannelRouter::find_route`（BFS 流动性感知寻路） |
| 路由节点激励（Routing Fee） | CR+ 引力权重加成：路由贡献 → `S_cog` 信用评分上升 |
| 链下清算（Off-chain Clearing） | `DailyNettingProcessor::clear`（多边净额轧账） |
| 链上结算（On-chain Settlement） | Solana Anchor 合约 / Quorum 暗池（`ChannelSettlementRecord` → 批量上链） |

---

## 3. 架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                     L2: PoCC 协作网络                                │
│  Agent-A ──[Intent]──► Agent-B ──[SubTask]──► Agent-C               │
└──────────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│               L1: GOAT 闪电通道网络（链下）                          │
│                                                                      │
│  A ═══[Ch A-B, 20 LIFE++]═══ B ═══[Ch B-C, 20 LIFE++]══════ C      │
│  ║                                                           ║       │
│  ╚══════════[Ch A-C, 5 LIFE++]════════════════════════════╝        │
│                                                                      │
│  • AgentLightningChannel：双向余额，PQC 签名，nonce 防重放           │
│  • HtlcPayment：SHA-256 哈希锁，原像揭示 → 原子结算                 │
│  • LightningChannelRouter：BFS 流动性感知多跳路由                    │
└──────────────────────────────────────────────────────────────────────┘
         │  日终关闭通道
         ▼
┌─────────────────────────────────────────────────────────────────────┐
│              L3a: DailyNettingProcessor（多边净额轧账）              │
│                                                                      │
│  1000 条通道 × N 笔支付  →  per-DID 净头寸  →  SettlementBatch      │
│  batch_hash = SHA-256(epoch ‖ sorted net positions)                 │
└──────────────────────────────────────────────────────────────────────┘
         │  批量上链
         ▼
┌─────────────────────────────────────────────────────────────────────┐
│              L3b: Solana Anchor / Quorum 结算合约                    │
│                                                                      │
│  只需提交 net_positions，而非原始百万笔微支付记录                    │
│  ZK-Compressor 可进一步将批次压缩为单个 SNARK 证明                  │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 4. 支付流程（端到端）

### 4.1 建立通道

Agent A（编排者）与 Agent B（执行者）在 Solana 上锁定担保金，同时在本地初始化 `AgentLightningChannel`：

```rust
let cid = router.open_channel(
    "agent-a.ny.ahin.io",
    "robot-b.shenzhen.ahin.io",
    50.0,   // A 锁定 50 LIFE++
    10.0,   // B 锁定 10 LIFE++ (对等担保)
).unwrap();
```

### 4.2 HTLC 微支付（单跳）

Agent A 每完成一个子任务单元，向 B 发起一笔 HTLC：

```rust
// A 随机生成原像，将 preimage 加密传递给 B（通过 Kyber-1024 隧道）
let preimage = b"random_secret_42";
// expiry_blocks = 144 ≈ 24 小时（Bitcoin 10 分钟出块 × 144）
let htlc = HtlcPayment::new(preimage, 0.05, 144, "robot-b.shenzhen.ahin.io");
channel.add_htlc(htlc).unwrap();

// B 完成做功，揭示原像，解锁资金
let earned = channel.settle_htlc(preimage).unwrap();
```

### 4.3 多跳路由（跨 Agent 协作）

当 A 需要支付给 C，但 A-C 之间没有直接通道时：

```rust
let route = router.find_route("A", "C", 5.0).unwrap();
// 返回: ["A", "B", "C"]

router.send_payment_along_route(&route, preimage, 5.0, 144).unwrap();
// B 作为路由节点：先从 A-B 通道收款，再从 B-C 通道转发
// B 的路由贡献计入 S_cog 信用评分，使其 CR+ 引力权重上升
```

### 4.4 日终轧账与结算

```rust
// 1. 关闭通道，产生结算记录
let record = router.close_channel(&cid, 50.0).unwrap();

// 2. 将差额注入 DailyNettingProcessor
let ctx_tx = CognitiveTransaction { ... settlement: SettlementInstruction { amount: record.net_delta.abs(), ... } };
netting_processor.ingest(ctx_tx);

// 3. 执行多边净额轧账
let batch = netting_processor.clear(epoch);
// batch.batch_hash 作为上链凭证 → Solana / Quorum 结算合约
```

---

## 5. 路由节点激励（GOAT 特色）

GOAT 闪电网络的核心创新是 **AI Agent 作为自主路由决策者**，而非被动转发节点。在 Life++ 中：

\[
\mathrm{Gravity}_{i \to j}(t) \mathrel{+}=
\alpha_{\mathrm{route}} \cdot \mathrm{RoutingVolume}_j(t)
\]

- 路由节点 B 的 `S_cog` 信用分随路由成功次数提升。
- `DynamicTrustRouter` 将路由贡献折算为 CR+ 引力加成，使更多任务自动流向高路由能力节点。
- 恶意路由（锁定 HTLC 后拒绝转发）触发 `SoulboundSlasher` 的全额惩没。

---

## 6. 安全属性

| 属性 | 机制 |
|---|---|
| 原子性 | HTLC 哈希锁保证"要么两端都结算，要么退款" |
| 防重放 | `nonce` 单调递增，PQC Falcon-1024 签名绑定通道状态 |
| 抗量子 | 通道握手使用 Kyber-1024（KEM）+ Falcon-1024（签名） |
| 超时保护 | `expiry_blocks` 到期后 `fail_htlc` 自动退款 |
| 双花防护 | Solana 链上锁定担保金，关闭通道须提交最新 `nonce` |
| 路由防女巫 | CR+ 二次方质押权重 + 拓扑熵惩罚 |

---

## 7. 实现位置

| 模块 | 路径 | 职责 |
|---|---|---|
| `AgentLightningChannel` | `crates/silicon-economy-layer/src/lightning/mod.rs` | 双向状态通道、HTLC 管理 |
| `LightningChannelRouter` | `crates/silicon-economy-layer/src/lightning/mod.rs` | BFS 多跳路由、全网通道图 |
| `DailyNettingProcessor` | `crates/silicon-economy-layer/src/netting/mod.rs` | 多边净额轧账、批次哈希 |
| `X402Channel` (PQC 握手) | `ahin-node/src/x402_channel.rs` | 量子安全通道开启握手 |
| `X402MemoryChannel` (高频) | `1_ahin_nervous_system/src/swarm_x402_simulator.rs` | 内存级微秒结算 |
| Solana 结算合约 | `3_thermodynamic_ledger/programs/` | 净头寸上链锚定 |
| Quorum 暗池结算 | `8_quorum_settlement/contracts/` | 机构级私有批次结算 |
