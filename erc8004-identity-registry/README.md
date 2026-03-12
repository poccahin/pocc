# erc8004-identity-registry

`SiliconIdentity_ERC8004.sol` 提供一个面向 ERC-8004 风格机器身份的最小可组合注册中心：

- ERC-721 智能体身份铸造（Persona Continuity）
- Agent Card 元数据存储（domain/capabilitiesURI/paymentWallet）
- 授权审计节点写入 PoCC 反馈与结算体量
- Soulbound Slash 黑名单机制与不可逆信誉清零
- 链上 `S_cog` 评分估算接口（`computeScog`）

> 合约依赖 OpenZeppelin (`ERC721`, `Ownable`)，推荐部署在 EVM L2 或高频侧链。
