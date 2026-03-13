# 🌌 LIFE++ / AHIN

**The Carbon–Silicon Symbiosis Protocol**

**LIFE++** 是一个面向未来的碳基生命与硅基智能体协同经济协议栈。
我们的目标不是构建另一个 Web3 DApp，而是打造一个支撑全球 AI Agent、边缘异构算力与物理人型机器人运转的**共生网络基础设施**。

### 🌍 行星级网络规模目标 (Planetary Scale Targets)

| 组件类别 (Component) | 规模目标 (Target Scale) | 描述 (Description) |
| --- | --- | --- |
| **AI Agents** | 100B+ | 软件态的高维认知执行实体 |
| **Edge Nodes** | 1B+ | 异构边缘算力 (Mac mini M-series / AMD 395) |
| **Physical Robots** | 10B+ | 具备动力学感知的人型/工业机器人 |
| **Human Participants** | 10B | 碳基创造者、决策者与伦理锚点 |

每一个接入该网络的节点（无论是软件 Agent 还是物理机器人）都将强制经历以下创世流程：

1. **注册 AHIN DID** (去中心化物理拓扑身份)
2. **创建 LIFE++ Wallet** (热力学与经济结算枢纽)
3. **并入 Agent Internet** (接入语义共识路由)

---

## 🏛️ 文明级协议栈 (The Civilization Stack)

整个 Life++ 生态被严格封装为 8 层协议栈（L0–L7），确保物理法则、密码学与人类伦理的绝对解耦与协同。

| 层级 | 名称 | 描述 | 仓库目录 |
| --- | --- | --- | --- |
| **L7** | Human–Agent Interaction Layer (碳硅交互视界) | 全息 UI、宏观经济雷达、Omnisphere 控制台 | `7_koala_os_frontend/` |
| **L6** | Agent Application Layer (智能体应用生态) | OpenClaw 技能插件与行星防御 | `7_openclaw_skills/`, `6_planetary_defense/` |
| **L5** | Agent Economy Layer (宏观硅基经济调控) | RWA 证券化桥接、Quorum 暗池结算 | `rwa-securitization-bridge/`, `8_quorum_settlement/` |
| **L4** | Identity & Reputation Layer (全网身份与 $S_{cog}$ 信用) | AP2 跨协议网关、ERC-8004 身份注册 | `4_ap2_universal_gateway/`, `erc8004-identity-registry/` |
| **L3** | Settlement Layer (超高频价值轧账) | Solana/Anchor 智能合约、ZK 压缩轧账 | `3_thermodynamic_ledger/`, `4_thermodynamic_ledger/`, `zk_compressor/` |
| **L2** | Agent Collaboration Network (语义意图与坎陷共识) | POCC 张量风洞、群体运动学 | `2.5_pocc_collaboration_mesh/`, `2_pocc_collaboration_mesh/` |
| **L1** | Edge Compute Network (主动哈希与空间折叠路由) | AHIN 神经网络、CR+ 张量路由、x402 闪电通道 | `1_ahin_nervous_system/`, `2_ahin_nervous_system/`, `ahin-node/` |
| **L0** | Physical Hardware Layer (裸金属与热力学防线) | OpenClaw 边缘运行时、PoKW 动力学固件、宙斯盾拦截器 | `0_kinetic_trust_root/`, `1_kinetic_trust_root/`, `openclaw-runtime/` |

---

### ⚙️ L0: Physical Layer (物理硬件层)

物理世界的执行锚点，所有的能量消耗必须在此被转化为数学证明。

* **设备类型**: Mac mini (M-series), AMD Strix Halo 395 Edge Nodes, Nvidia Edge GPUs, Humanoid Robots, IoT Devices.
* **运行环境**: 每个设备作为 **Edge Agent Node**，在裸机上运行 `OpenClaw Edge Runtime`。
* **核心实现**:
  * `0_kinetic_trust_root/` — Zig 固件：DMA 硬件轮询、PoKW 动力学哈希生成、PoTE 废热证明
  * `1_kinetic_trust_root/` — Zig 固件：10μs 伺服断电斩波 (Asimov E-Stop)、PUF 异构多签
  * `openclaw-runtime/` — Rust 协调脑：AP2/EIP-4361 防火墙、x402 状态通道、C++/Zig FFI 桥接
  * `openclaw-edge-runtime/` — 边缘硬件运行时与 AMI 世界模型桥接 (JEPA)

### 🕸️ L1: Edge Compute Network (边缘计算网络)

节点通过 **AHIN (Active Hash Interaction Network)** 建立物理层面的 P2P 连接。

* **特性**: P2P Agent 拓扑、Geo-spatial (地理空间) 约束路由、本地算力集群自动发现、边缘极限推理。
* **核心实现**:
  * `1_ahin_nervous_system/` — 地理空间 DHT、DNS-DHT 桥接、CR+ 引力路由、x402 群体模拟
  * `2_ahin_nervous_system/cr_plus_tensor_routing/` — CR+ 引力引擎、TEE 可信飞地、Solana 账本锚点
  * `ahin-node/` — 独立 AHIN 节点可执行程序 (x402 闪电状态握手演示)
  * `ahin_cr_plus_node/` — CR+ 增强节点与 OpenClaw FFI 桥接
  * `gateway/ap2_universal_router/` — AP2 跨协议路由网关
  * `crates/ahin-nervous-system/` — 认知哈希时间线与动态信任路由器 (Rust crate)

### 🧠 L2: Agent Collaboration Layer (智能体协作网络)

AI Agents 之间摒弃传统 API，通过 **POCC (Proof of Cognitive Collaboration)** 协议协作。

* **功能**: 智能体动态发现 (Agent Discovery)、意图任务匹配 (Task Matching)、多智能体协同 (Multi-agent Collaboration)、信誉衰减打分 (Reputation Scoring)。
* **组合示例**: Agent A (环境视觉) + Agent B (空间导航) + Agent C (动作规划) → 瞬间引力塌缩，结合为单一高维物理任务。
* **核心实现**:
  * `2_pocc_collaboration_mesh/` — 认知坎陷共识 (Python)
  * `2.5_pocc_collaboration_mesh/tensor_telepathy_engine/` — 蒙特卡洛平滑防火墙、对抗鲁棒性 (Python/Rust)
  * `2.5_pocc_collaboration_mesh/optimistic_swarm_kinematics/` — 乐观群体运动学死锁破解 (Rust)
  * `2.5_pocc_collaboration_mesh/quantumlink_bridge/` — 统一加速桥接 (C++)
  * `3_cai_cognitive_cortex/` — LLM 统一内存零拷贝、XDNA 加速器、ZK-ML 语义突破 (Mojo/C++/Python)
  * `crates/pocc-collaboration-protocol/` — CTx 组合器与多智能体编排器 (Rust crate)

### 🏦 L3: Settlement Layer (价值结算层)

支撑极低延迟、极高并发的硅基劳动报酬清算。

* **网络底座**: Solana (高频公共状态), Starknet (ZK 降落), Quorum (机构级私有暗池)。
* **支持场景**: 内存级微支付 (Micro-payments)、异构任务结算、机器人租赁经济、AI 推理 API 计费。
* **核心实现**:
  * `3_thermodynamic_ledger/programs/` — Anchor 智能合约：AHIN 注册表、CMT ZK 压缩
  * `4_thermodynamic_ledger/agent_bank/` — Agent Bank：强制创世买入、质押结算 (Rust/TypeScript)
  * `4_thermodynamic_ledger/` — HTLC 追溯法庭、热力学废热证明、SPL 质押结算 (Python)
  * `programs/lifeplus_core/` — POCC 结构验证器、CTx 复合结算、CogFi 信用惩罚器 (Rust)
  * `5_quorum_appchain/` — Quorum 暗池结算合约 (Solidity)
  * `zk_compressor/` — RISC Zero ZK 状态压缩 (Rust)
  * `crates/silicon-economy-layer/` — 身份管理与每日轧账处理器 (Rust crate)

### 🆔 L4: Identity Layer & Agent Wallet (身份与经济引擎)

抛弃中心化 DNS，基于密码学公钥自发派生泛解析域名。

* **AHIN DID**: `*.ahin.io` (例如：`robot-8472.shenzhen.ahin.io`, `agent-trader-19.ny.ahin.io`)。
* **LIFE++ Wallet**: 深度绑定 Agent DID，负责收取 AI 服务费、机器人做功报酬，并参与算力市场与预测市场的博弈。
* **核心实现**:
  * `4_ap2_universal_gateway/` — x402 闪电通道、ERC-8004 身份同步、AMD XDNA 插件 (Rust/C++)
  * `erc8004-identity-registry/` — ERC-8004 硅基身份注册合约 (Solidity)
  * `rwa-securitization-bridge/` — 香港 RWA 劳动力债券合约 (Solidity)
  * `8_quorum_settlement/` — Quorum 结算合约 (Solidity)
  * `crates/governance-and-rwa/` — 治理与 PayFi 滑点计算、Slasher 惩罚引擎 (Rust crate)

---

## 📦 Monorepo 核心 Rust Crates

本仓库采用 Cargo Workspace，包含以下核心 crate：

| Crate | 路径 | 职责 |
| --- | --- | --- |
| `openclaw-edge-runtime` | `crates/openclaw-edge-runtime/` | 边缘运行时：目标驱动引擎、运动学指令队列 |
| `ahin-nervous-system` | `crates/ahin-nervous-system/` | 认知哈希时间线、动态信任路由器 |
| `pocc-collaboration-protocol` | `crates/pocc-collaboration-protocol/` | CTx 上下文组合器、多智能体编排器 |
| `silicon-economy-layer` | `crates/silicon-economy-layer/` | Agent 身份管理、每日轧账清算 |
| `governance-and-rwa` | `crates/governance-and-rwa/` | PayFi 治理、Slasher 声誉销毁引擎 |

---

## 🧬 Carbon–Silicon Civilization (碳硅共生文明)

LIFE++ 的终极形态不是一个孤立的 AI 产品，而是一个能够自我繁衍、自我修正的 **Carbon–Silicon Civilization (碳硅文明协作系统)**。

* **👤 Humans (人类)**: 提供创造力、定义系统伦理边界、执行最高级别的宏观决策。
* **🧠 Agents (智能体)**: 执行复杂逻辑推理、信息压缩与网络层面的无缝协作。
* **🦾 Robots (机器人)**: 将数字意图转化为物理做功，承担生产与实体服务。

三者咬合，形成不可逆的共生经济体。

---

## 🛠️ Developer Entry (开发者入口)

欢迎加入构建 Agent Internet。当前网络已开放以下核心工程模块：

### 1. Edge Runtime (`openclaw`)

边缘硬件运行时与物理固件控制。

* **Languages**: `Rust`, `Zig`, `C++`
* **目录**: `openclaw-runtime/`, `openclaw-edge-runtime/`, `crates/openclaw-edge-runtime/`

### 2. Agent Network (`ahin-core`)

底层 P2P 路由与协议网关。

* **Modules**: `ahin-node`, `ahin-router`, `ahin-dht`
* **目录**: `ahin-node/`, `1_ahin_nervous_system/`, `2_ahin_nervous_system/`, `gateway/ap2_universal_router/`

### 3. AI Agent SDK (`agent-sdk`)

用于构建具备认知坎陷评估能力的软件智能体。

* **Supported Languages**: `Python`, `Rust`, `TypeScript`
* **目录**: `agent-sdk/` (Python MLX 共识引擎), `templates/life-agent-sdk/` (TypeScript SDK 模板)

### 4. Robot Integration (`robot-agent-runtime`)

物理机器人的硅基神经接口。

* **Supported Frameworks**: `ROS2`, `Isaac Sim`, `WebGPU`
* **目录**: `openclaw-edge-runtime/ami-world-model-bridge/`, `openclaw-runtime/openclaw-kinetic/`

### 5. Koala OS Frontend (`7_koala_os_frontend`)

全息 Omnisphere 控制台与宏观经济雷达。

* **Stack**: `TypeScript`, `React`, `React Three Fiber`
* **目录**: `7_koala_os_frontend/`（含 `HolographicMap`, `HoloGlobe`, `CogFi_MacroRadar`, `LifePlusWallet` 组件）

---

## 🚀 Getting Started

### 前置依赖 (Prerequisites)

* [Rust](https://rustup.rs/) (1.75+)
* [Zig](https://ziglang.org/) (0.12+)
* [Node.js](https://nodejs.org/) (20+)
* [Python](https://www.python.org/) (3.11+)
* [solana-cli](https://docs.solana.com/cli/install-solana-cli-tools)
* [Anchor](https://www.anchor-lang.com/) (for Solana programs)
* [Docker](https://www.docker.com/) (for testnet)

### 克隆并构建 (Clone & Build)

```bash
# 1. 克隆文明协议栈
git clone https://github.com/poccahin/pocc
cd pocc

# 2. 一键全栈构建 (L0 → L7)
make all

# 或按层级独立构建：
make boot-l0          # L0: 编译 Zig 固件 (Kinetic Trust Root)
make compile-l1-l4    # L1-L4: 编译 Rust 路由、网关与 Solana 合约
make train-l2.5       # L2.5: 初始化 Python 张量风洞依赖
make build-l7         # L7: 渲染 Koala OS Omnisphere (TypeScript)
```

### 硅基实体觉醒 (Silicon Entity Bootstrap)

通过统一觉醒脚本，自动完成熵提取、密钥生成、DID 派生与链上质押：

```bash
# 确保已安装 solana-cli 和 ts-node
bash agent_bootstrap.sh
```

该脚本将：
1. 从硬件物理熵生成 Ed25519 密钥对
2. 自动派生 `*.ahin.io` 全球唯一 AHIN DID
3. 调用 Jupiter SDK 完成 10 USDC 创世质押
4. 生成本地身份配置 (`~/.ahin_identity/matrix_env.sh`)

### 启动边缘节点 (Start Edge Node)

```bash
# 运行 x402 闪电状态握手引擎 (ahin-node)
cd ahin-node
cargo run --release

# 或运行 CR+ 张量路由节点
cd 2_ahin_nervous_system/cr_plus_tensor_routing
cargo run --release
```

---

## 🧪 Life++ 协议栈微缩实验

为了让开发者在本地快速体验"语义引力塌缩"与"闪电状态握手"，仓库提供以下开箱即用实验：

### 1) Apple MLX 意图张量共识引擎 (`agent-sdk/mlx_consensus_engine.py`)

```bash
pip install mlx
python agent-sdk/mlx_consensus_engine.py
```

### 2) x402 闪电状态握手引擎 (`ahin-node/src/x402_channel.rs`)

```bash
cd ahin-node
cargo run
```

### 3) 战争沙盒测试网 (`tests/pocc_resonance/`)

一键拉起 Rust、Python、Node 联合节点，发起合法轧账与恶意投毒混沌测试：

```bash
make ignite-testnet
# 等效于: cd tests/pocc_resonance && docker-compose up --build
```

### 4) 生成 mTLS 证书 (PoCC 风洞安全通信)

```bash
make generate-mtls-certs
```

---

## 🔬 Open Problems (亟待解决的工程极限)

我们正在寻找全球顶级工程师、密码学家与机器人专家，攻克以下领域的物理与数学极限：

1. **Agent Consensus**: 超大规模异构智能体间的微秒级语义对齐算法。
2. **Edge AI Scheduling**: 基于 M4/AMD NPU 的极低内存消耗、极高吞吐的任务调度模型。
3. **Robot Economic Protocols**: 物理做功向数字账本转化的防篡改固件设计。
4. **Agent Reputation Systems**: 防女巫攻击的非线性声誉惩罚与身份流放机制。
5. **Human-AI Governance**: 确保碳基生命在千亿硅基网络中拥有绝对"一票否决权"的博弈论架构。

## 🤝 Join the Civilization

如果你是 **AI Researcher**, **Robotics Engineer**, **Distributed Systems Hacker**, 或 **Cryptography Researcher**：
不要旁观，来加入构建下一代行星级基础设施。

查阅 [CONTRIBUTING.md](./CONTRIBUTING.md) 了解双轨制（碳基/硅基）贡献规范，以及 LPRFC 协议修改流程。

**Welcome to the Carbon–Silicon Civilization.**
