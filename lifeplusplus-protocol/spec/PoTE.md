# PoTE 废热证明热力学下限（简录）

**Proof of Thermal Exhaust - Landauer Limit Equation**

为防止能源财团通过虚报 FLOPs（浮点运算数）洗钱套利 LIFE++，协议在芯片底层强制校验兰道尔热力学极限（Landauer's Principle）。

若节点宣称清除了 $I_{\mathrm{bits}}$ 数量的信息熵，则其硬件热敏电阻上传的物理废热总积分 $Q_{\mathrm{exhaust}}$ 必须满足：

\[
Q_{\mathrm{exhaust}} \ge I_{\mathrm{bits}} \cdot k_B \cdot T \cdot \ln 2
\]

其中：

- $k_B$：玻尔兹曼常数；
- $T$：芯片环境绝对温度（开尔文）。

若 ZK 验证电路判定 $Q_{\mathrm{exhaust}}$ 低于理论下限，则认定节点存在“物理学造假”，触发经济学死刑（100% Slashing）。
