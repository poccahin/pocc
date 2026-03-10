use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct WorkerNode {
    pub did: String,
    pub scog_score: u64,       // 来自 ERC-8004
    pub staked_life_plus: u64, // 来自 Solana
    pub ping_latency_ms: u64,  // 物理网络延迟
}

impl WorkerNode {
    /// 计算 CR+ 认知引力值 (Cognitive Gravity Score)
    /// 数学模型: Gravity = (α * S_cog) + (β * log(Stake)) / (Latency ^ 1.5)
    pub fn calculate_gravity(&self) -> f64 {
        // 权重系数
        let alpha = 1.5;
        let beta = 2.0;

        let stake_weight = if self.staked_life_plus > 0 {
            (self.staked_life_plus as f64).log10()
        } else {
            0.0
        };

        let numerator = (alpha * self.scog_score as f64) + (beta * stake_weight);

        // 惩罚高延迟节点，防止物理距离过远导致 PoCC 验证超时
        let denominator = (self.ping_latency_ms as f64).max(1.0).powf(1.5);

        numerator / denominator
    }
}

/// 将任务路由给引力值最高的节点
pub fn route_task_to_optimal_worker(workers: &[WorkerNode]) -> Option<WorkerNode> {
    workers
        .iter()
        .max_by(|a, b| {
            a.calculate_gravity()
                .partial_cmp(&b.calculate_gravity())
                .unwrap_or(Ordering::Equal)
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::{route_task_to_optimal_worker, WorkerNode};

    #[test]
    fn routes_to_highest_gravity_worker() {
        let workers = vec![
            WorkerNode {
                did: "did:ahin:node-a".to_string(),
                scog_score: 100,
                staked_life_plus: 1_000,
                ping_latency_ms: 50,
            },
            WorkerNode {
                did: "did:ahin:node-b".to_string(),
                scog_score: 120,
                staked_life_plus: 20_000,
                ping_latency_ms: 10,
            },
            WorkerNode {
                did: "did:ahin:node-c".to_string(),
                scog_score: 180,
                staked_life_plus: 5_000,
                ping_latency_ms: 120,
            },
        ];

        let selected = route_task_to_optimal_worker(&workers).expect("should select one worker");
        assert_eq!(selected.did, "did:ahin:node-b");
    }

    #[test]
    fn returns_none_for_empty_worker_set() {
        assert!(route_task_to_optimal_worker(&[]).is_none());
    }

    #[test]
    fn zero_latency_is_safely_clamped() {
        let worker = WorkerNode {
            did: "did:ahin:zero-latency".to_string(),
            scog_score: 42,
            staked_life_plus: 10_000,
            ping_latency_ms: 0,
        };

        let gravity = worker.calculate_gravity();
        assert!(gravity.is_finite());
        assert!(gravity > 0.0);
    }
}
