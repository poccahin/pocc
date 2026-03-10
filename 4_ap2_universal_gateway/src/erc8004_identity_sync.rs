use ethers::prelude::*;
use std::sync::Arc;

// ERC-8004 智能体身份注册中心 ABI 简写
abigen!(
    Erc8004Registry,
    r#"[
        function getAgentScogScore(address agent) external view returns (uint256)
        function isPersonaActive(address agent) external view returns (bool)
    ]"#
);

pub struct IdentityOracle {
    eth_client: Arc<Provider<Http>>,
    registry_contract: Erc8004Registry<Provider<Http>>,
}

impl IdentityOracle {
    pub async fn new(
        rpc_url: &str,
        contract_address: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let client = Arc::new(provider);
        let address = contract_address.parse::<Address>()?;

        Ok(Self {
            eth_client: client.clone(),
            registry_contract: Erc8004Registry::new(address, client),
        })
    }

    /// 跨链查验：查询以太坊主网上的 $S_{cog}$ 信用分
    pub async fn verify_cross_chain_identity(&self, eth_address: &str) -> Result<u64, String> {
        let agent_addr = eth_address
            .parse::<Address>()
            .map_err(|_| "Invalid ETH address")?;

        // 1. 检查以太坊端该身份是否已被销毁 (Soulbound Slash)
        let is_active = self
            .registry_contract
            .is_persona_active(agent_addr)
            .call()
            .await
            .map_err(|e| format!("RPC Error: {}", e))?;

        if !is_active {
            return Err("❌ [ERC-8004] Agent Persona is burned/inactive on Ethereum.".into());
        }

        // 2. 读取链上声誉资本
        let scog_score: U256 = self
            .registry_contract
            .get_agent_scog_score(agent_addr)
            .call()
            .await
            .map_err(|e| format!("RPC Error: {}", e))?;

        println!(
            "🔗 [Oracle] ERC-8004 Identity Synced. Address: {}, S_cog: {}",
            eth_address, scog_score
        );

        if scog_score > U256::from(u64::MAX) {
            return Err("S_cog overflow: value does not fit into u64".into());
        }

        Ok(scog_score.as_u64())
    }

    pub fn provider(&self) -> Arc<Provider<Http>> {
        self.eth_client.clone()
    }
}
