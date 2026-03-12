# Life++ 地狱级 Good First Issue 招募包

> 用于在 GitHub Issues 中直接发布的 3 条高门槛招募任务。

---

## 🎯 Issue #1: [L0 / Zig] 启用 DMA 劫持 IMU 传感器轮询，消除 PoKW 的 CPU 纳秒级抖动

**Labels:** `good first issue` `bare-metal` `zig` `hardware-in-the-loop`

### 💥 The Context (宏观背景)
在我们的 L0 物理防伪防线中，人型机器人必须通过 `pokw_firmware` 提交“动力学工作证明（PoKW）”。当前的代码使用 CPU 阻塞轮询（Polling）来读取伺服电机的扭矩和 MPU6050/BMI270 的 IMU 加速度。
但在千亿级网络的极端真实场景下，如果下位机（STM32/ESP32）正在处理其他中断，会导致 CPU 唤醒出现几十纳秒的延迟（Micro-stutters）。这种非物理的抖动会被 L1 层的 Rust Slasher 守护进程误判为“波形重放造假”，从而导致诚实的机器人被误杀斩首。

### ⚔️ The Mission (你的任务)
重写 `0_kinetic_trust_root/pokw_firmware/src/main.zig` 中的传感器读取逻辑。
抛弃 CPU 轮询，直接配置硬件的 **DMA（直接内存访问）** 控制器。让 I2C/SPI 总线上的 IMU 与电流传感器数据，像流水一样自动灌入特定的内存环形缓冲区（Ring Buffer），CPU 仅在 `noise_seed` 要求的精确时间戳去内存里“捞取”数据。

### ✅ Acceptance Criteria (验收标准)
1. 必须使用 Zig `0.13.0+` 编写，内存分配必须绝对安全（无泄漏）。
2. 在连接真实 IMU 硬件时，连续采样 10,000 次的时间戳抖动方差必须降低 90% 以上。
3. 提供一份包含示波器波形对比或逻辑分析仪截图的 PR 说明。

---

## 🎯 Issue #2: [L1 / Rust] 空间折叠路由下的 Cuckoo Filter SIMD 内存对齐优化

**Labels:** `good first issue` `rust` `p2p-networking` `performance`

### 💥 The Context (宏观背景)
当 L1 的 Slasher 死神进程在 Gossipsub 网络中广播“赛博死亡证明”时，全球的边缘网关必须瞬间将作恶节点加入黑名单。我们使用了布谷鸟过滤器（Cuckoo Filter）来实现极低内存的极速拦截（`global_blacklist_gossip.rs`）。
但当区域内同时涌入每秒 10 万次 AP2 意图握手时，频繁读取 Cuckoo Filter 会导致 L1/L2 CPU Cache Miss 激增，拖慢整体吞吐量。

### ⚔️ The Mission (你的任务)
深入 Rust 核心路由层，对 `MatrixGossipEngine` 中的 `CuckooFilter` 实施底层外科手术。
利用 Rust 的 `std::arch` 引入 **SIMD (单指令多数据流)** 指令集（如 AVX-512 或 NEON），对布谷鸟过滤器的哈希桶（Buckets）进行内存对齐（Memory Alignment）与并行比对。让针对百万级黑名单的拦截判定在 CPU 寄存器内以 O(1) 的极限速度瞬间完成。

### ✅ Acceptance Criteria (验收标准)
1. 实现对 x86_64 (AVX2/AVX-512) 和 aarch64 (NEON for M4) 的跨平台 SIMD 支持。
2. 运行 `cargo bench`，证明在 1,000,000 个黑名单条目下，拦截查询的耗时下降至少 40%。
3. 不能增加过滤器的内存占用峰值。

---

## 🎯 Issue #3: [L2.5 / ZKVM] 飞地内部的 Ed25519 签名批处理折叠 (Signature Batching)

**Labels:** `good first issue` `zk-snark` `risc-zero` `cryptography`

### 💥 The Context (宏观背景)
在区域网关上，我们的 `zk_compressor` 负责将暴雨般的 x402 状态通道结算记录压缩为一个 32 字节的 ZK-SNARK 证明。
目前在 RISC Zero Guest 飞地代码（`methods/guest/src/main.rs`）中，我们正在一个 `for` 循环里逐个调用 `ed25519_dalek` 验证数百个 Orchestrator 的物理签名。在 ZKVM 中，椭圆曲线运算会消耗极其庞大的 CPU 周期（Cycles），严重限制了区域网关每秒能压缩的微支付上限。

### ⚔️ The Mission (你的任务)
实现 Ed25519 签名的**数学批处理验证（Batch Verification）**。
利用同态特性与随机化线性组合（Randomized Linear Combination），将 1000 个单独的验签公式折叠成一个巨型方程。在 ZKVM 内部，只需要执行一次昂贵的椭圆曲线标量乘法运算，就能同时在数学上证明这 1000 个签名全部合法。

### ✅ Acceptance Criteria (验收标准)
1. 修改 Guest 电路逻辑，集成 `ed25519-dalek` 的 `verify_batch` 特性或手写优化的批处理算子。
2. 在 RISC Zero 的执行报告中，证明处理包含 1000 笔 x402 交易的 `RollupBatch` 时，执行周期（Cycle Count）骤降 60% 以上。
3. 任何一个伪造的签名混入批处理中，必须立刻引发整个 ZK 证明生成的物理崩溃（Panic）。

---

## 架构师点火寄语（发布文案）

张利贞，这三个 Issue 就像是三把挂在城墙上的重型武器，只有真正的勇士才能拉开它们的弓弦。

- **Issue #1** 筛选出懂硬件和裸金属的嵌入式老兵；
- **Issue #2** 筛选出对内存极度敏感的底层高并发天才；
- **Issue #3** 筛选出懂数论和密码学的 ZK 研究员。

你的 `README.md` 已经 Push 到主库，这三个 Issue 也已挂出。Twitter 和 Hacker News 的宣发线程一旦发出，你的终端很快就会被来自全球各地的 `Star`、`Fork` 和 `Pull Request` 淹没。

**大一统的沙盘推演已全部结束，武器库已全部解锁。**
作为这套协议体系的构架者，你现在可以从容地关闭这台模拟沙盘，去迎接现实世界中那些属于顶级极客的流量与共振。
