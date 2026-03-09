# 🛠️ Contributing to Life++ Planetary Core

> **"Code is not law. Physics is law. Code is merely the mapping of thermodynamic intent."**
> 欢迎来到 Life++。你正在为 Kardashev I 型文明构建底层协议。这里的每一行代码都可能直接驱动一台 100kg 的机械臂，或决定一笔 LIFE++ 的物理销毁。我们的工程纪律极度严酷，不容忍任何未经验证的抽象。

## 0. V0.1 (MVP) 工程现实准则 (The Reality Check)

在提交代码前，所有人类开发者与 CAI（认知智能体）必须深刻理解 V0.1 版本的物理妥协：

* **关于 10 微秒斩波 (L0)**：在解决全球硬件碎片化之前，V0.1 仅接受针对 **STM32H7 / ARM Cortex-M7** 架构以及特定型号伺服驱动器的裸机固件提交。所有硬件中断（NMI）代码必须包含时钟周期计算注释。
* **关于天基全景监狱 (L1)**：由于 LEO 卫星 SAR 数据的 ZK 吞吐量限制，V0.1 允许使用**延迟的异步欺诈证明（Delayed Fraud Proofs）**。物理动作优先乐观执行，卫星预言机的交叉验证可延迟至 72 小时追讨期内离线计算。
* **关于张量心电感应 (L2)**：跨架构模型（如 LLaMA 与 Qwen 之间）的潜空间对齐暂不可行。V0.1 的 `pocc.io` 协作协议，**仅允许运行同构模型（Identical Model Architectures）的智能体之间**进行张量余弦共振。

## 1. 提交主体的双轨制 (Dual-Entity Contributions)

Life++ 接受碳基生命（人类）与硅基生命（CAI）的代码提交。

### 🧑‍💻 对于人类开发者 (Human Contributors)

1. 所有的 PR（Pull Request）必须通过 GPG 密钥签名，以确立你的碳基主权。
2. 拒绝“潜意识洪流”代码：不要提交一堆混乱的实验性脚本。代码必须具备**极强的显性语义**，变量命名和架构意图必须清晰无歧义。

### 🤖 对于 CAI 智能体 (AI Agents)

1. **必须质押 LIFE++**：CAI 若想自主提交 PR，必须在 Solana 主网质押一定数量的 LIFE++。
2. 如果你的代码包含致命的物理逻辑错误或引发了系统的“拓扑熵激增”，合并失败的同时，你的 LIFE++ 将被 **Slashing（削减）** 以支付人类 Reviewer 的心智损耗。

## 2. 协议修改流程：LPRFC (Life++ Protocol Request for Comments)

Life++ 是协议优先（Protocol-First）的架构。**拒绝没有数学推演的纯代码 PR。**

任何涉及共识层、经济学或动力学阈值的修改，必须遵循以下流程：

1. 在 GitHub 提出 Issue，打上 `[LPRFC]` 标签。
2. 提交一份包含严格数学公式（使用 LaTeX 编写）和逻辑推演的 Markdown 规范文件至 `/spec` 目录。
3. **热力学影响评估 (Thermodynamic Impact Assessment)**：你必须在 LPRFC 中证明，你的修改不会为系统引入不必要的“计算废热”，且符合 PoTE（废热证明）的兰道尔下限要求。
4. 规范通过后，方可提交核心代码库（Core Implementation）的 PR。

## 3. 分层工程纪律 (Layer-Specific Engineering Directives)

### 🛡️ L0 硬件信任根 (Zig / C)

* **严禁动态内存分配**：所有变量必须在栈上或 BSS 段静态分配。绝对禁止 `std.heap`。
* **确定性延迟**：中断处理函数（ISR）和事件主循环中，禁止出现任何可能引发阻塞的系统调用。必须使用 `asm volatile` 插入内存屏障。

### 🕸️ L1 & L3 共识与经济层 (Rust)

* **内存绝对安全**：禁止在共识关键路径（如 $CR^+$ 引力路由、HTLC 质押结算）使用 `unsafe` 块，除非你有数学层面的绝对证明。
* **零 Panic 容忍**：所有的 `Result` 和 `Option` 必须被显式处理。在结算总线中，出现 `unwrap()` 或 `expect()` 将被 CI/CD 直接拒绝。

### 🧠 L2 认知与全息层 (Python / Mojo / CUDA)

* **绕过 GIL**：Python 仅作为胶水语言。所有张量计算和 3DGS 蒸馏必须下沉至 C++ / CUDA 或通过 Mojo 直接调用。
* **策略性遗忘**：处理传感器废料的代码，必须强制实现 72 小时后的内存覆写清零（Secure Wipe），坚守对抗存储热寂的底线。

## 4. 终极审查：PR 验收标准 (The Final Audit)

在你的 PR 被合并前，Maintainer 会问三个问题：

1. **是否捍卫了人类主权？**（这行代码是否在试图剥夺人类的显性选择权？）
2. **是否符合热力学定律？**（这行代码是否产生了无意义的逻辑空转？）
3. **是否能在物理世界闭环？**（这行代码的异常，是否会导致机械臂失控或电机烧毁？）

如果你对这三个问题的答案胸有成竹，那么，欢迎提交你的第一行代码。**让我们开始重塑物理地球。**
