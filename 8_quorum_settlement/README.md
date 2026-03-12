# Quorum 终极结算层（L3.5）

该目录提供 Quorum AppChain 上的终极结算合约：

- `contracts/LifePlus_Quorum_Settlement.sol`
  - 验证由 ZK-Compressor 生成的证明。
  - 更新全球状态根（`globalStateRoot`）。
  - 支持 Quorum/Tessera 隐私飞地模式下的私有批次锚定事件。

## Quorum 隐私交易调用示意

```javascript
const txOptions = {
  from: regionalGatewayAddress,
  to: lifePlusQuorumSettlementAddress,
  data: contract.methods.anchorPrivateSettlement(batchHash).encodeABI(),
  privateFor: ["BULeR8JyUWhiUWiyK3Xz8QxY7/a/Jq0aU="],
};
```

> 注意：`privateFor` 为 Quorum 特有字段。交易会进区块，但仅指定 Tessera 公钥节点可解密。
