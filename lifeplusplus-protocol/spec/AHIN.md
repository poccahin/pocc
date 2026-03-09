# AHIN 与 CR+ 张量引力路由规范

**Active Hash Interaction Network & ChainRank+ Tensor Routing Specification**

## 1. 摘要 (Abstract)

在 9000 亿个 CAI 组成的网络中，传统 DHT（分布式哈希表）或 Gossip 协议会因广播风暴而失效；而基于算力加权的 DPoS（委托权益证明）又会快速滑向寡头化。AHIN 协议引入**引力张量路由（Gravity Tensor Routing）**与 **CR+（ChainRank Plus）**反马太效应算法，确保任务始终被路由给“最合适且未被垄断”的降熵节点。

## 2. CR+ 数学模型 (The Mathematical Model of CR+)

CR+ 表示节点的“综合降熵引力质量”。其目标是放弃“富者愈富”的单调累积逻辑，引入二次方衰减与拓扑熵惩罚。

节点 $j$ 在时间 $t$ 对节点 $i$ 的路由引力定义为：

\[
\mathrm{Gravity}_{i \to j}(t) =
\frac{\alpha\,\Delta S_j(t) + \beta\,\sqrt{\mathrm{Stake}_j}}
{D_{i,j}^{2}\,\exp\!\big(\gamma\,H_{\mathrm{topo}}(j)\big)}
\]

### 变量释义与反垄断机制

- **$\Delta S_j(t)$（降熵总值）**：节点 $j$ 历史验证通过的物理与精神降熵总和。真抓实干的节点引力更高。
- **$\mathrm{Stake}_j$（二次方质押）**：节点 $j$ 在 Solana 合约中质押的 LIFE++ 数量，使用 $\sqrt{\mathrm{Stake}_j}$ 进入模型。这是反马太效应核心（类比二次方投票 / Quadratic Funding）：质押 10000 枚代币的财团，其路由引力约仅为质押 100 枚节点的 10 倍，而非 100 倍。
- **$D_{i,j}$（语义与空间距离）**：节点 $i,j$ 在高维意图张量空间（Semantic Space）与物理 GPS 空间（Physical Space）的加权欧氏距离。
- **$H_{\mathrm{topo}}(j)$（拓扑熵/垄断惩罚）**：若节点 $j$ 在过去 24h 接单过多并成为中心化枢纽，其局部拓扑熵上升；分母中的指数惩罚会快速削弱其引力，迫使任务回流边缘长尾节点。
- **$\alpha,\beta,\gamma$（系统超参数）**：用于平衡“真实降熵贡献”“经济承诺强度”“反垄断惩罚强度”。

## 3. 路由共识 (Routing Consensus)

当用户提交任务意图（Intent）时，系统不进行全网广播。节点 $i$ 仅向 $\mathrm{Gravity}_{i \to j}$ 值最高的前 $k$ 个候选节点发起握手，并在局部拓扑中滚动扩散。

该机制在近似 $O(\log N)$ 的复杂度下实现行星级网络自平衡，避免广播风暴与算力资本买断路由权。
