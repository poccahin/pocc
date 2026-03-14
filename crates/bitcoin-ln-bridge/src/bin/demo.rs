//! # Bitcoin Lightning Network × Life++ 端到端演示
//!
//! 运行方式（在仓库根目录）：
//! ```bash
//! cargo run --bin demo -p bitcoin-ln-bridge
//! ```
//!
//! ## 演示流程
//!
//! ```text
//! Scene 1: 人类用户通过 BTC 闪电网络向智能体支付任务报酬
//!   └── 创建 BOLT-11 发票 → 用户支付 → 网关原子换汇 → 智能体获得 LIFE++
//!
//! Scene 2: 多跳闪电支付（Alice → Bob → Carol 三节点网络）
//!   └── 路径发现 → 路由费计算 → 逐跳 HTLC → 接收方揭示原像 → 结算
//!
//! Scene 3: 通道关闭与结算报告
//!   └── 合作关闭 → 最终余额快照 → 模拟上链提交
//!
//! Scene 4: 适用性评估摘要
//!   └── 打印 Bitcoin LN 在 Life++ 中三类场景的适用等级
//! ```

use bitcoin_ln_bridge::{
    channel::BtcLightningChannel,
    gateway::BtcLnGateway,
    invoice::invoice_summary,
    router::{EdgePolicy, LnRouter},
};

// ─── 固定测试向量（实际生产中必须使用密码学安全随机数生成器）─────────────────────────

/// 支付原像（32 字节随机秘密）
const PREIMAGE_USER_TO_AGENT: [u8; 32] = [
    0x7f, 0x3a, 0x12, 0xde, 0xad, 0xbe, 0xef, 0x00,
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
];

/// 多跳支付原像
const PREIMAGE_MULTIHOP: [u8; 32] = [
    0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
    0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
    0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
    0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
];

fn main() {
    print_banner();
    scene1_human_to_agent_payment();
    scene2_multihop_routing();
    scene3_channel_close();
    scene4_applicability_assessment();
    println!("\n{}", "═".repeat(70));
    println!("✅  全部演示完成。Bitcoin LN × Life++ 框架运行正常。");
    println!("{}", "═".repeat(70));
}

// ─────────────────────────────────────────────────────────────────────────────
// Scene 1: 人类用户 BTC 入金 → 智能体获得 LIFE++
// ─────────────────────────────────────────────────────────────────────────────

fn scene1_human_to_agent_payment() {
    println!("\n{}", "─".repeat(70));
    println!("🔌  SCENE 1: 人类用户通过 Bitcoin Lightning 向 Life++ 智能体付款");
    println!("{}", "─".repeat(70));

    let mut gateway = BtcLnGateway::new(
        "gateway.btcln.ahin.io",
        50_000_000,  // 0.5 BTC
        10_000.0,    // 10,000 LIFE++
        10_000,      // 1 LIFE++ = 10,000 sat（假设 BTC = $100,000，1 LIFE++ ≈ $1）
        3,           // 0.3% 手续费
    );

    println!("\n  🏦  网关节点: {}", gateway.node_id);
    println!("  💰  BTC 流动性: {} sat ({:.4} BTC)", gateway.btc_liquidity_sat, gateway.btc_liquidity_sat as f64 / 1e8);
    println!("  💎  LIFE++ 流动性: {:.0} LIFE++", gateway.life_liquidity);
    println!("  📈  汇率: 1 LIFE++ = {} sat", gateway.sats_per_life_token);

    // 智能体任务：机器人仓库清洁，报酬 5 LIFE++
    let task_life_amount = 5.0_f64;
    let agent_did = "robot-warehouse-001.shenzhen.ahin.io";
    let now = 1_741_000_000_u64;  // 固定时间戳（2025 年）

    println!("\n  📋  任务: 仓库清洁 | 报酬: {task_life_amount} LIFE++ | 执行方: {agent_did}");

    // 网关为此次任务生成 BOLT-11 发票
    let invoice = gateway.create_invoice(
        task_life_amount,
        agent_did,
        "Warehouse-001 Cleaning Task — Life++",
        3600,
        &PREIMAGE_USER_TO_AGENT,
        now,
    ).expect("Failed to create invoice");

    println!("\n  🧾  生成 BOLT-11 发票:");
    println!("      {}", invoice_summary(&invoice));
    println!("      金额: {} msat ({} sat)", invoice.amount_msat, invoice.amount_sats());

    // 用户支付（模拟 BTC LN 支付已完成，揭示 preimage）
    println!("\n  👤  [用户] 通过 BTC 闪电钱包支付 {} sat ...", invoice.amount_sats());
    println!("  🔑  [用户] 原像已揭示，BTC 侧结算完成");

    let receipt = gateway.on_ramp(
        &invoice,
        &PREIMAGE_USER_TO_AGENT,
        agent_did,
        invoice.amount_msat,
        now + 60,
    ).expect("on_ramp failed");

    println!("\n  ✅  [网关] 原子换汇完成:");
    println!("      Swap ID: {}", receipt.swap_id);
    println!("      BTC 收到: {} sat", receipt.btc_sat);
    println!("      LIFE++ 分发给智能体: {:.4} LIFE++", receipt.life_tokens);
    println!("      智能体: {}", receipt.agent_did);
    println!("\n  💰  网关当前余额: BTC={} sat, LIFE++={:.2}", gateway.btc_liquidity_sat, gateway.life_liquidity);
}

// ─────────────────────────────────────────────────────────────────────────────
// Scene 2: 多跳闪电路由（Alice → Bob → Carol）
// ─────────────────────────────────────────────────────────────────────────────

fn scene2_multihop_routing() {
    println!("\n{}", "─".repeat(70));
    println!("⚡  SCENE 2: 多跳闪电支付（Alice → Bob → Carol）");
    println!("{}", "─".repeat(70));

    let mut router = LnRouter::new();

    // 构建三节点网络
    let ch_ab = BtcLightningChannel::open(
        "alice.ln.ahin.io",
        "bob.ln.ahin.io",
        1_000_000,  // Alice 侧 0.01 BTC
        500_000,    // Bob 侧
    );
    let ch_bc = BtcLightningChannel::open(
        "bob.ln.ahin.io",
        "carol.ln.ahin.io",
        800_000,    // Bob 侧
        200_000,    // Carol 侧
    );

    // Alice ──[A-B]──► Bob ──[B-C]──► Carol
    let cid_ab = ch_ab.channel_id.clone();
    let cid_bc = ch_bc.channel_id.clone();

    router.add_channel(ch_ab,
        EdgePolicy { base_fee_sat: 1, fee_rate_ppm: 100, available_liquidity_sat: 1_000_000 },
        EdgePolicy { base_fee_sat: 1, fee_rate_ppm: 100, available_liquidity_sat: 500_000 },
    ).unwrap();

    router.add_channel(ch_bc,
        EdgePolicy { base_fee_sat: 1, fee_rate_ppm: 150, available_liquidity_sat: 800_000 },
        EdgePolicy { base_fee_sat: 1, fee_rate_ppm: 150, available_liquidity_sat: 200_000 },
    ).unwrap();

    println!("\n  🌐  网络拓扑:");
    println!("      alice.ln ──[{:.8}...]── bob.ln ──[{:.8}...]── carol.ln", cid_ab, cid_bc);

    let payment_amount = 50_000_u64;  // 50,000 sat = 0.0005 BTC
    println!("\n  💸  Alice 向 Carol 发起支付: {} sat", payment_amount);
    println!("      Alice 与 Carol 之间无直接通道，需通过 Bob 转发");

    // 路径发现
    let route = router.find_route("alice.ln.ahin.io", "carol.ln.ahin.io", payment_amount)
        .expect("Route not found");

    println!("\n  🗺️   发现路径 ({} 跳):", route.hops.len());
    for (i, hop) in route.hops.iter().enumerate() {
        println!("      Hop {}: {} | 转发 {} sat | 路由费 {} sat | HTLC 超时 {} 块",
            i + 1, hop.node_id, hop.amount_sat, hop.fee_sat, hop.expiry_blocks);
    }
    println!("      总金额: {} sat | 总路由费: {} sat ({:.4}%)",
        route.total_amount_sat, route.total_fee_sat,
        route.total_fee_sat as f64 / payment_amount as f64 * 100.0);

    // 执行支付
    println!("\n  🔐  [Alice] 锁定 HTLC 并发送洋葱包...");
    println!("  🔐  [Bob]   转发 HTLC（收取路由费）...");
    println!("  🔑  [Carol] 揭示原像，结算 HTLC...");

    let received = router.execute_payment(&route, &PREIMAGE_MULTIHOP)
        .expect("Payment failed");

    println!("\n  ✅  支付成功! Carol 收到: {} sat", received);

    // 显示通道余额变化
    let ch = router.channel(&cid_bc).unwrap();
    println!("\n  📊  支付后 Bob-Carol 通道余额:");
    println!("      Bob(local): {} sat | Carol(remote): {} sat",
        ch.local_balance_sat, ch.remote_balance_sat);
}

// ─────────────────────────────────────────────────────────────────────────────
// Scene 3: 通道关闭与结算报告
// ─────────────────────────────────────────────────────────────────────────────

fn scene3_channel_close() {
    println!("\n{}", "─".repeat(70));
    println!("🔒  SCENE 3: 通道关闭与链上结算");
    println!("{}", "─".repeat(70));

    // 模拟一段时间的多次支付后关闭
    let mut ch = BtcLightningChannel::open(
        "agent-orchestrator.ahin.io",
        "robot-executor-007.sz.ahin.io",
        500_000,  // 编排者锁定 500,000 sat
        100_000,  // 执行者锁定 100,000 sat
    );

    println!("\n  📂  通道开启:");
    println!("      {} ↔ {}", ch.local_node_id, ch.remote_node_id);
    println!("      初始容量: {} sat | 本地: {} sat | 远端: {} sat",
        ch.total_capacity_sat(), ch.local_balance_sat, ch.remote_balance_sat);

    // 模拟 5 次微支付
    let task_preimages: Vec<[u8; 32]> = (0u8..5).map(|i| [i + 1; 32]).collect();
    let task_amounts = [10_000u64, 15_000, 8_000, 12_000, 20_000];

    println!("\n  ⚙️   执行 5 个子任务微支付:");
    for (i, (preimage, amount)) in task_preimages.iter().zip(task_amounts.iter()).enumerate() {
        ch.add_htlc(preimage, *amount, 144, &ch.remote_node_id.clone()).unwrap();
        ch.fulfill_htlc(preimage).unwrap();
        println!("      任务 {}: {} sat 已结算 ✓", i + 1, amount);
    }

    let total_paid: u64 = task_amounts.iter().sum();
    println!("\n  💰  累计支付: {} sat", total_paid);
    println!("  📊  当前余额: 本地={} sat | 远端={} sat | 承诺号={}",
        ch.local_balance_sat, ch.remote_balance_sat, ch.commitment_number);

    // 合作关闭
    println!("\n  🤝  发起合作关闭 (cooperative close)...");
    let close_record = ch.cooperative_close().unwrap();

    println!("\n  ✅  通道已关闭，提交链上结算:");
    println!("      通道 ID: {}", close_record.channel_id);
    println!("      {} 最终获得: {} sat ({:.6} BTC)",
        close_record.local_node_id, close_record.local_final_sat,
        close_record.local_final_sat as f64 / 1e8);
    println!("      {} 最终获得: {} sat ({:.6} BTC)",
        close_record.remote_node_id, close_record.remote_final_sat,
        close_record.remote_final_sat as f64 / 1e8);
    println!("\n  ⛓️   [模拟] 向 Solana / Quorum 结算合约提交净头寸...");
    println!("      ✓ 链上结算完成（实际需调用 Anchor 合约 anchor_settlement）");
}

// ─────────────────────────────────────────────────────────────────────────────
// Scene 4: Bitcoin LN 适用性评估
// ─────────────────────────────────────────────────────────────────────────────

fn scene4_applicability_assessment() {
    println!("\n{}", "─".repeat(70));
    println!("📋  SCENE 4: Bitcoin Lightning Network 在 Life++ 中的适用性评估");
    println!("{}", "─".repeat(70));

    println!(r#"
  ┌─────────────────────────────────────────────────────────────────┐
  │  Bitcoin LN 核心优势                                            │
  ├─────────────────────────────────────────────────────────────────┤
  │  ✅ 最广泛信任的去中心化价值存储（BTC 是全球最大加密资产）       │
  │  ✅ 经战场验证的 HTLC 实现（BOLT-2/4/11，运行 7+ 年）           │
  │  ✅ 亚秒级链下结算（通道内支付时延 < 500ms）                    │
  │  ✅ 全球流动性网络（公共节点 > 15,000，通道容量 > 5,000 BTC）   │
  │  ✅ 原子跨链互换（BTC LN ↔ LIFE++ via HTLC atomic swap）       │
  ├─────────────────────────────────────────────────────────────────┤
  │  Bitcoin LN 局限性                                              │
  ├─────────────────────────────────────────────────────────────────┤
  │  ❌ 链上开/关通道需要 10 分钟确认（高频智能体协作不适用）        │
  │  ❌ Bitcoin 脚本有限，不支持复杂条件结算                        │
  │  ❌ 无智能合约原语（无法直接执行 PoCC/PoTE 逻辑）               │
  │  ❌ 通道管理复杂度高（流动性再平衡、路由节点选择）              │
  └─────────────────────────────────────────────────────────────────┘"#);

    println!(r#"
  ┌─────────────────────────────────────────────────────────────────┐
  │  Life++ 场景适用性矩阵                                          │
  ├──────────────────────────────────────────┬──────────────────────┤
  │  场景                                    │  适用等级            │
  ├──────────────────────────────────────────┼──────────────────────┤
  │  人类用户 → Agent 入金（BTC 支付任务）   │  ✅ 强烈推荐         │
  │  跨国/跨机构大额结算（>0.1 BTC）         │  ✅ 适用             │
  │  智能体-智能体纳秒级微支付               │  ❌ 不适用（用 x402）│
  │  PoCC 联名质押与惩没                     │  ❌ 不适用（用 Solana）│
  │  机器人做功热力学报酬（毫聪级）          │  🔶 部分适用          │
  │  RWA 债券结算（需合规 KYC）              │  🔶 部分适用（+LSP） │
  │  多 Agent 协作日终净额结算               │  ✅ 适用（通过网关）  │
  └──────────────────────────────────────────┴──────────────────────┘"#);

    println!(r#"
  ┌─────────────────────────────────────────────────────────────────┐
  │  推荐架构：三层支付分层                                         │
  ├─────────────────────────────────────────────────────────────────┤
  │  L1: Bitcoin Lightning Network                                  │
  │      • 人类用户 BTC 入金 (on-ramp)                             │
  │      • 全球跨机构结算层（BTC 计价）                             │
  │      • BtcLnGateway 桥接 BTC ↔ LIFE++                          │
  │                                                                 │
  │  L2: Life++ x402 闪电通道（AgentLightningChannel）             │
  │      • Agent-to-Agent 微秒级 LIFE++ 支付                       │
  │      • 每日轧账（DailyNettingProcessor）                       │
  │                                                                 │
  │  L3: Solana / Quorum 链上结算                                   │
  │      • 净头寸上链锚定                                           │
  │      • ZK 压缩批量证明                                          │
  └─────────────────────────────────────────────────────────────────┘"#);

    println!(r#"
  📐  实施建议：
      1. 部署 BtcLnGateway 节点，接入比特币主网 LN（推荐 LDK 或 LND）
      2. 网关持有双侧流动性池：BTC (LN) + LIFE++ (Solana)
      3. 使用 Pyth/Switchboard 预言机实时更新 BTC/LIFE++ 汇率
      4. HTLC 同一 preimage 保证换汇原子性（无信任）
      5. 日均结算走 DailyNettingProcessor → 单笔 Solana 交易"#);
}

// ─────────────────────────────────────────────────────────────────────────────
// 辅助
// ─────────────────────────────────────────────────────────────────────────────

fn print_banner() {
    println!("{}", "═".repeat(70));
    println!("⚡  Bitcoin Lightning Network × Life++  端到端演示");
    println!("    框架版本: v0.1 | 比特币闪电网络适用性评估 + 可运行代码");
    println!("{}", "═".repeat(70));
}
