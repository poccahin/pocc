use rand::Rng;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

/// 模拟 5 维的意图/能力张量空间
pub type Tensor = [f64; 5];

/// 计算两个张量之间的认知摩擦力 (基于余弦相似度)
fn calculate_cognitive_friction(tensor_a: &Tensor, tensor_b: &Tensor) -> f64 {
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..5 {
        dot_product += tensor_a[i] * tensor_b[i];
        norm_a += tensor_a[i] * tensor_a[i];
        norm_b += tensor_b[i] * tensor_b[i];
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 1.0;
    }

    let alignment = dot_product / (norm_a.sqrt() * norm_b.sqrt());
    1.0 - alignment // 摩擦力 = 1 - 相似度
}

#[derive(Clone, Debug)]
pub struct ActiveHashIntent {
    pub intent_id: String,
    pub semantic_vector: Tensor,
    pub max_friction_allowed: f64,
}

/// 边缘智能体 (Worker)
pub struct AgentNode {
    pub agent_id: String,
    pub capability_tensor: Tensor,
}

impl AgentNode {
    /// 潜意识评估引擎：捕获主动哈希并计算认知坎陷
    pub async fn evaluate_and_react(
        &self,
        mut ahin_receiver: broadcast::Receiver<ActiveHashIntent>,
    ) {
        while let Ok(intent) = ahin_receiver.recv().await {
            // 模拟不同节点的物理网络延迟与处理时间
            let latency = rand::thread_rng().gen_range(5..50);
            sleep(Duration::from_millis(latency)).await;

            // 计算语义摩擦力
            let friction =
                calculate_cognitive_friction(&self.capability_tensor, &intent.semantic_vector);

            // 坎陷判定
            if friction <= intent.max_friction_allowed {
                println!(
                    "✨ [COLLAPSE] Agent {} resonated! Friction: {:.4} (Threshold: {}). Sending ACK.",
                    self.agent_id, friction, intent.max_friction_allowed
                );
            } else {
                // 静默抛弃，避免全网广播风暴 (这是 AHIN 的高明之处)
                // println!("... Agent {} ignored. Friction {:.4} too high.", self.agent_id, friction);
            }
        }
    }
}

// =====================================================================
// 创世实验室主程序：引爆 1000 个智能体的微缩宇宙
// =====================================================================
pub async fn run_swarm_simulation() {
    println!("🌌 [LABORATORY] Initializing AHIN Swarm Simulator...");

    // 建立一个能够承载 1000 个消息的 P2P 广播信道
    let (ahin_sender, _) = broadcast::channel::<ActiveHashIntent>(1000);
    let mut handles = vec![];

    // 1. 瞬间生成 1000 个边缘智能体
    println!("🧬 [LABORATORY] Spawning 1000 Edge Agents with random Cognitive Basins...");
    for i in 0..1000 {
        let receiver = ahin_sender.subscribe();
        let mut rng = rand::thread_rng();

        let agent = AgentNode {
            agent_id: format!("Worker-{:04}", i),
            // 随机生成该智能体的心智能力张量
            capability_tensor: [
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
            ],
        };

        // 让智能体进入异步监听状态
        handles.push(tokio::spawn(async move {
            agent.evaluate_and_react(receiver).await;
        }));
    }

    sleep(Duration::from_millis(500)).await;

    // 2. 投掷 (Throwing)：Orchestrator 抛出极其苛刻的主动哈希意图
    let orchestrator_intent = ActiveHashIntent {
        intent_id: "INTENT_X402_PRECISION_ASSEMBLY".to_string(),
        semantic_vector: [0.95, 0.85, 0.10, 0.05, 0.20], // 假设这是一个高精度物理装配任务
        max_friction_allowed: 0.05,                      // 摩擦力必须极小（仅允许 5% 的认知偏差）
    };

    println!("\n☄️  [THROWING] Orchestrator drops Active Hash into the mesh...");
    println!(
        "🎯 Target Vector: {:?}, Max Friction: {}",
        orchestrator_intent.semantic_vector, orchestrator_intent.max_friction_allowed
    );

    ahin_sender.send(orchestrator_intent).unwrap();

    // 等待共振与塌缩发生
    sleep(Duration::from_secs(2)).await;
    println!("\n✅ [LABORATORY] Swarm stabilization complete. Observers may check the console.");

    // 避免未使用变量告警，显式释放句柄集合
    drop(handles);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friction_is_zero_for_identical_tensors() {
        let t = [0.3, 0.4, 0.5, 0.6, 0.7];
        let friction = calculate_cognitive_friction(&t, &t);
        assert!((friction - 0.0).abs() < 1e-10);
    }

    #[test]
    fn friction_defaults_to_max_when_zero_vector_present() {
        let zero = [0.0, 0.0, 0.0, 0.0, 0.0];
        let non_zero = [1.0, 0.0, 0.0, 0.0, 0.0];
        let friction = calculate_cognitive_friction(&zero, &non_zero);
        assert_eq!(friction, 1.0);
    }
}
