/// 智能体多维资产与状态快照
#[derive(Debug, Clone)]
pub struct AgentStateSnapshot {
    pub did: String,
    /// 纯粹的金融质押金 (例如 LIFE++ 代币余额)
    pub financial_stake_tokens: f64,
    /// 该节点自创世以来累积的物理做功证明 (以焦耳为单位)
    pub historical_pokw_joules: f64,
    /// L3 层计算出的防女巫认知评分 (TrustRank)
    pub scog_score: f64,
}

/// 弹性带宽与访问权控制器
#[derive(Debug, Clone)]
pub struct ElasticBandwidthController {
    /// 预言机提供的实时币价
    pub token_price_usd_oracle: f64,
    /// 基础分配带宽
    pub base_bandwidth_mbps: f64,
    /// 资本衰减系数 (遏制马太效应)
    pub capital_decay_factor: f64,
    /// 热力学权重
    pub thermodynamic_weight: f64,
}

impl ElasticBandwidthController {
    pub fn new(oracle_price: f64) -> Self {
        Self {
            token_price_usd_oracle: oracle_price,
            base_bandwidth_mbps: 1.0,
            capital_decay_factor: 10.0,
            thermodynamic_weight: 0.05,
        }
    }

    /// 核心算法：计算智能体的“综合网络访问权 (Access Power)”
    pub fn compute_access_power(&self, agent: &AgentStateSnapshot) -> f64 {
        let token_price = self.token_price_usd_oracle.max(0.0);
        let fiat_value_usd = agent.financial_stake_tokens.max(0.0) * token_price;

        // 对数衰减保障：1 + x/k 必须为正
        let decay = self.capital_decay_factor.max(f64::EPSILON);
        let capital_power = (1.0 + fiat_value_usd / decay).ln();

        // 物理做功按线性增益，负值被防御性归零
        let physical_power =
            agent.historical_pokw_joules.max(0.0) * self.thermodynamic_weight.max(0.0);

        // 认知评分不允许导致负乘数
        let cognitive_multiplier = (1.0 + agent.scog_score).max(0.0);

        (capital_power + physical_power) * cognitive_multiplier
    }

    /// 执行网络带宽与接单队列分配
    pub fn allocate_network_resources(&self, agents: &[AgentStateSnapshot]) -> Vec<(String, f64)> {
        let mut allocations = Vec::with_capacity(agents.len());

        for agent in agents {
            let power = self.compute_access_power(agent);
            let allocated_bandwidth = self.base_bandwidth_mbps + (power * 2.5);
            let final_bandwidth = allocated_bandwidth.clamp(self.base_bandwidth_mbps, 1000.0);

            allocations.push((agent.did.clone(), final_bandwidth));

            #[cfg(debug_assertions)]
            println!(
                "🌐 [BANDWIDTH] Agent {} | Access Power: {:.2} | Allocated: {:.2} Mbps",
                agent.did, power, final_bandwidth
            );
        }

        allocations
    }

    /// 价格休克熔断保护机制 (Price Shock Resilience)
    pub fn handle_market_crash(&mut self, new_price: f64) {
        if new_price < self.token_price_usd_oracle * 0.5 {
            println!(
                "🚨 [ORACLE WARNING] Massive price drop detected. Activating Thermodynamic Shield."
            );
            self.thermodynamic_weight *= 2.0;
        }

        self.token_price_usd_oracle = new_price.max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentStateSnapshot, ElasticBandwidthController};

    #[test]
    fn logarithmic_decay_limits_capital_dominance() {
        let controller = ElasticBandwidthController::new(1.0);

        let low = AgentStateSnapshot {
            did: "low".into(),
            financial_stake_tokens: 100.0,
            historical_pokw_joules: 0.0,
            scog_score: 0.0,
        };

        let whale = AgentStateSnapshot {
            did: "whale".into(),
            financial_stake_tokens: 1_000_000.0,
            historical_pokw_joules: 0.0,
            scog_score: 0.0,
        };

        let low_power = controller.compute_access_power(&low);
        let whale_power = controller.compute_access_power(&whale);

        assert!(whale_power > low_power);
        assert!(whale_power / low_power < 5.0);
    }

    #[test]
    fn thermodynamic_component_survives_price_crash() {
        let mut controller = ElasticBandwidthController::new(2.0);

        let agent = AgentStateSnapshot {
            did: "edge-node".into(),
            financial_stake_tokens: 5.0,
            historical_pokw_joules: 100.0,
            scog_score: 0.1,
        };

        let before = controller.compute_access_power(&agent);
        controller.handle_market_crash(0.4);
        let after = controller.compute_access_power(&agent);

        assert!(controller.thermodynamic_weight >= 0.1);
        assert!(after > 0.0);
        assert!(after > before);
    }

    #[test]
    fn allocation_has_floor_and_ceiling() {
        let controller = ElasticBandwidthController::new(1.0);

        let constrained = AgentStateSnapshot {
            did: "tiny".into(),
            financial_stake_tokens: 0.0,
            historical_pokw_joules: 0.0,
            scog_score: 0.0,
        };

        let huge = AgentStateSnapshot {
            did: "huge".into(),
            financial_stake_tokens: 1_000_000_000.0,
            historical_pokw_joules: 1_000_000.0,
            scog_score: 10.0,
        };

        let allocations = controller.allocate_network_resources(&[constrained, huge]);
        assert_eq!(allocations[0].1, controller.base_bandwidth_mbps);
        assert_eq!(allocations[1].1, 1000.0);
    }
}
