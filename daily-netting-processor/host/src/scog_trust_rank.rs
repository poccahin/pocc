use std::collections::HashMap;

/// 智能体节点在图网络中的数据结构。
#[derive(Debug, Clone)]
pub struct AgentNode {
    pub did: String,
    /// 物理做功锚点（Thermodynamic Anchor）。
    pub pokw_burn_usd: f64,
    /// 节点对外发起的成功 CTx（出度）。
    pub outbound_edges: HashMap<String, f64>,
    /// 当前迭代的 S_cog 分数。
    pub current_scog: f64,
}

impl AgentNode {
    fn new(did: &str) -> Self {
        Self {
            did: did.to_string(),
            pokw_burn_usd: 0.0,
            outbound_edges: HashMap::new(),
            current_scog: 0.0,
        }
    }
}

/// 基于马尔可夫链的反女巫 S_cog 评分引擎。
#[derive(Debug)]
pub struct ScogGraphEngine {
    pub nodes: HashMap<String, AgentNode>,
    /// 阻尼系数（通常为 0.85）。
    pub damping_factor: f64,
    /// 迭代收敛精度。
    pub convergence_threshold: f64,
    /// 最大幂迭代轮数。
    pub max_iterations: usize,
}

impl Default for ScogGraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScogGraphEngine {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            damping_factor: 0.85,
            convergence_threshold: 1e-9,
            max_iterations: 200,
        }
    }

    /// 构建网络拓扑：将每日轧差中的认知交易（CTx）转化为有向图边。
    pub fn add_cognitive_transaction(&mut self, payer_did: &str, payee_did: &str, volume_usd: f64) {
        if volume_usd <= 0.0 || payer_did == payee_did {
            return;
        }

        let payer = self
            .nodes
            .entry(payer_did.to_string())
            .or_insert_with(|| AgentNode::new(payer_did));

        *payer
            .outbound_edges
            .entry(payee_did.to_string())
            .or_insert(0.0) += volume_usd;

        self.nodes
            .entry(payee_did.to_string())
            .or_insert_with(|| AgentNode::new(payee_did));
    }

    /// 设置物理锚点：黑客可以伪造身份，但无法伪造电费单。
    pub fn set_thermodynamic_anchor(&mut self, did: &str, burn_usd: f64) {
        let node = self
            .nodes
            .entry(did.to_string())
            .or_insert_with(|| AgentNode::new(did));
        node.pokw_burn_usd = burn_usd.max(0.0);
    }

    /// 执行热力学 TrustRank 幂迭代（Power Iteration），返回实际迭代轮数。
    pub fn compute_global_scog(&mut self) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }

        let node_count = self.nodes.len() as f64;
        let mut total_pokw_burn = 0.0;

        for node in self.nodes.values_mut() {
            total_pokw_burn += node.pokw_burn_usd;
            node.current_scog = 1.0 / node_count;
        }

        for iteration in 1..=self.max_iterations {
            let mut next_scog: HashMap<String, f64> = HashMap::with_capacity(self.nodes.len());
            let mut dangling_mass = 0.0;

            // 第一阶段：信任沿有向边流动。
            for payer_node in self.nodes.values() {
                let total_outbound_volume: f64 = payer_node.outbound_edges.values().sum();

                if total_outbound_volume > 0.0 {
                    for (payee_did, volume) in &payer_node.outbound_edges {
                        let trust_flow = payer_node.current_scog * (volume / total_outbound_volume);
                        *next_scog.entry(payee_did.clone()).or_insert(0.0) += trust_flow;
                    }
                } else {
                    // 悬空节点：先累计其质量，随后按 anchor 分布二次分配。
                    dangling_mass += payer_node.current_scog;
                }
            }

            let mut max_diff = 0.0_f64;
            let mut mass_sum = 0.0;

            // 第二阶段：阻尼 + 热力学基线 + 悬空质量注入。
            for (did, node) in &mut self.nodes {
                let incoming_trust = *next_scog.get(did).unwrap_or(&0.0);
                let baseline_trust = if total_pokw_burn > 0.0 {
                    node.pokw_burn_usd / total_pokw_burn
                } else {
                    1.0 / node_count
                };

                let dangling_trust = dangling_mass * baseline_trust;
                let new_score = (1.0 - self.damping_factor) * baseline_trust
                    + self.damping_factor * (incoming_trust + dangling_trust);

                mass_sum += new_score;
                max_diff = max_diff.max((new_score - node.current_scog).abs());
                node.current_scog = new_score;
            }

            // 数值稳定性：归一化到总质量 1.0。
            if mass_sum > 0.0 {
                for node in self.nodes.values_mut() {
                    node.current_scog /= mass_sum;
                }
            }

            if max_diff < self.convergence_threshold {
                return iteration;
            }
        }

        self.max_iterations
    }

    /// 提取可用于 PayFi 的安全授信额度。
    pub fn get_safe_credit_line(&self, did: &str, total_pool_usd: f64) -> f64 {
        self.nodes
            .get(did)
            .map(|node| node.current_scog * total_pool_usd.max(0.0))
            .unwrap_or(0.0)
    }

    /// 返回按 S_cog 由高到低排序的节点视图，用于聚合器落库或上链压缩前审计。
    pub fn rank_desc(&self) -> Vec<&AgentNode> {
        let mut nodes: Vec<&AgentNode> = self.nodes.values().collect();
        nodes.sort_by(|a, b| b.current_scog.total_cmp(&a.current_scog));
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::ScogGraphEngine;

    #[test]
    fn sybil_cartel_without_anchor_collapses() {
        let mut engine = ScogGraphEngine::new();

        // 主网（有锚点）
        engine.add_cognitive_transaction("anchor_1", "merchant_a", 200.0);
        engine.add_cognitive_transaction("merchant_a", "anchor_1", 20.0);
        engine.set_thermodynamic_anchor("anchor_1", 1000.0);

        // 女巫子图（内部疯狂刷单，但无锚点）
        for i in 0..50 {
            let from = format!("sybil_{i}");
            let to = format!("sybil_{}", (i + 1) % 50);
            engine.add_cognitive_transaction(&from, &to, 1_000_000.0);
        }

        engine.compute_global_scog();

        let sybil_total: f64 = engine
            .nodes
            .iter()
            .filter(|(did, _)| did.starts_with("sybil_"))
            .map(|(_, node)| node.current_scog)
            .sum();

        let anchor_score = engine.nodes.get("anchor_1").unwrap().current_scog;
        let merchant_score = engine.nodes.get("merchant_a").unwrap().current_scog;

        assert!(
            sybil_total < 1e-6,
            "sybil cluster should collapse, got {}",
            sybil_total
        );
        assert!(
            anchor_score + merchant_score > 0.99,
            "mainnet mass should dominate, got anchor={}, merchant={}",
            anchor_score,
            merchant_score
        );
    }

    #[test]
    fn safe_credit_line_is_proportional_to_scog() {
        let mut engine = ScogGraphEngine::new();
        engine.add_cognitive_transaction("a", "b", 10.0);
        engine.set_thermodynamic_anchor("a", 100.0);
        engine.compute_global_scog();

        let a_credit = engine.get_safe_credit_line("a", 10_000.0);
        let b_credit = engine.get_safe_credit_line("b", 10_000.0);

        assert!(a_credit > b_credit);
        assert!((a_credit + b_credit - 10_000.0).abs() < 1e-6);
    }
}
