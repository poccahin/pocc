use rand::Rng;
use sha2::{Digest, Sha256};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, Duration, Instant};

type Tensor = [f64; 5];

/// AHIN 主动哈希意图
#[derive(Clone, Debug)]
pub struct ActiveHashIntent {
    pub intent_id: String,
    pub semantic_vector: Tensor,
    pub max_friction_allowed: f64,
    pub total_bounty_life_plus: f64,
}

/// x402 内存状态通道
pub struct X402MemoryChannel {
    pub channel_id: String,
    pub nonce: u64,
    pub settled_balance: f64,
}

impl X402MemoryChannel {
    /// 模拟微秒级高频内存结算（零网络 IO，纯密码学更新）
    pub fn micro_settle(&mut self, amount: f64) -> String {
        self.nonce += 1;
        self.settled_balance += amount;

        let mut hasher = Sha256::new();
        hasher.update(self.channel_id.as_bytes());
        hasher.update(self.nonce.to_be_bytes());
        hasher.update(self.settled_balance.to_be_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// 边缘智能体（Worker）
pub struct AgentNode {
    pub agent_id: String,
    pub capability_tensor: Tensor,
}

impl AgentNode {
    pub async fn evaluate_and_react(
        &self,
        mut ahin_receiver: broadcast::Receiver<ActiveHashIntent>,
        ack_sender: mpsc::Sender<(String, ActiveHashIntent)>,
    ) {
        while let Ok(intent) = ahin_receiver.recv().await {
            let friction = self.calculate_friction(&intent.semantic_vector);

            if friction <= intent.max_friction_allowed {
                println!(
                    "✨ [CANXIAN COLLAPSE] Agent {} resonated! Friction: {:.4}. Sending x402 Handshake...",
                    self.agent_id, friction
                );
                let _ = ack_sender.send((self.agent_id.clone(), intent)).await;
            }
        }
    }

    fn calculate_friction(&self, target: &Tensor) -> f64 {
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..5 {
            dot += self.capability_tensor[i] * target[i];
            norm_a += self.capability_tensor[i].powi(2);
            norm_b += target[i].powi(2);
        }

        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0;
        }

        1.0 - (dot / (norm_a.sqrt() * norm_b.sqrt()))
    }
}

/// 创世实验室主程序：引爆 x402 闪电暴雨
pub async fn run_x402_swarm_simulation() {
    println!("🌌 [LABORATORY] Initializing AHIN + x402 Lightning Swarm...");

    let (ahin_sender, _) = broadcast::channel::<ActiveHashIntent>(1000);
    let (ack_tx, mut ack_rx) = mpsc::channel(100);

    // 1. 生成 1000 个边缘智能体
    for i in 0..1000 {
        let receiver = ahin_sender.subscribe();
        let ack_tx_clone = ack_tx.clone();
        let mut rng = rand::thread_rng();

        let agent = AgentNode {
            agent_id: format!("Worker-{i:04}"),
            capability_tensor: [
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
            ],
        };

        tokio::spawn(async move {
            agent.evaluate_and_react(receiver, ack_tx_clone).await;
        });
    }

    // 2. Orchestrator 抛出高价值复合认知任务
    let orchestrator_intent = ActiveHashIntent {
        intent_id: "INTENT_HEAVY_LIFTING".to_string(),
        semantic_vector: [0.90, 0.90, 0.10, 0.10, 0.10],
        max_friction_allowed: 0.03,
        total_bounty_life_plus: 5.0,
    };

    println!("\n☄️  [THROWING] Orchestrator drops 5.0 LIFE++ Intent into the mesh...");
    ahin_sender.send(orchestrator_intent).unwrap();

    // 3. 并发处理 ACK，并为每个 Worker 开启一条独立 x402 内存通道
    let mut active_channels = 0;

    let timeout = tokio::time::sleep(Duration::from_millis(100));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some((agent_id, intent)) = ack_rx.recv() => {
                active_channels += 1;

                tokio::spawn(async move {
                    println!("⚡ [x402 OPEN] Channel established with {} in 2.4ms.", agent_id);

                    let mut channel = X402MemoryChannel {
                        channel_id: format!("{}-{}", intent.intent_id, agent_id),
                        nonce: 0,
                        settled_balance: 0.0,
                    };

                    let start_time = Instant::now();

                    for _step in 1..=10 {
                        sleep(Duration::from_micros(500)).await;

                        let micro_pay = 0.005;
                        let state_hash = channel.micro_settle(micro_pay);

                        println!(
                            "   💸 [RAIN] {} | Nonce: {:02} | Paid: +{:.3} LIFE++ | State: {}...",
                            agent_id,
                            channel.nonce,
                            micro_pay,
                            &state_hash[0..8]
                        );
                    }

                    let duration = start_time.elapsed();
                    println!(
                        "🔒 [x402 CLOSE] {} completed task. Total Earned: {:.3} LIFE++. Time: {:?}",
                        agent_id,
                        channel.settled_balance,
                        duration
                    );
                });
            }
            _ = &mut timeout => {
                break;
            }
        }
    }

    sleep(Duration::from_secs(1)).await;
    println!(
        "\n✅ [LABORATORY] Storm passed. {} elite agents executed the intent.",
        active_channels
    );
}

#[cfg(test)]
mod tests {
    use super::{AgentNode, X402MemoryChannel};

    #[test]
    fn micro_settle_increments_nonce_and_balance() {
        let mut channel = X402MemoryChannel {
            channel_id: "INTENT_HEAVY_LIFTING-Worker-0001".to_string(),
            nonce: 0,
            settled_balance: 0.0,
        };

        let first = channel.micro_settle(0.005);
        let second = channel.micro_settle(0.005);

        assert_eq!(channel.nonce, 2);
        assert!((channel.settled_balance - 0.01).abs() < f64::EPSILON);
        assert_ne!(first, second);
    }

    #[test]
    fn friction_is_zero_for_identical_vectors() {
        let agent = AgentNode {
            agent_id: "Worker-0001".to_string(),
            capability_tensor: [0.90, 0.90, 0.10, 0.10, 0.10],
        };

        let friction = agent.calculate_friction(&[0.90, 0.90, 0.10, 0.10, 0.10]);
        assert!(friction.abs() < 1e-12);
    }
}
