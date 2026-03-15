use rand::Rng;
use sha2::{Digest, Sha256};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone)]
struct SimCognitiveTransaction {
    agent_pubkey: String,
    semantic_friction: f32,
    settled_volume: f64,
    signature: Vec<u8>,
}

impl SimCognitiveTransaction {
    fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.agent_pubkey.as_bytes());
        hasher.update(self.semantic_friction.to_be_bytes());
        hasher.update(self.settled_volume.to_be_bytes());
        hasher.update(&self.signature);

        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    }
}

#[derive(Debug, Default)]
struct ConcurrentMerkleTree {
    leaves: Vec<[u8; 32]>,
}

impl ConcurrentMerkleTree {
    fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, leaf: [u8; 32]) {
        self.leaves.push(leaf);
    }

    fn root(&self) -> [u8; 32] {
        if self.leaves.is_empty() {
            return [0u8; 32];
        }

        let mut level = self.leaves.clone();
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len().div_ceil(2));

            for pair in level.chunks(2) {
                let left = pair[0];
                let right = if pair.len() == 2 { pair[1] } else { pair[0] };

                let mut hasher = Sha256::new();
                hasher.update(left);
                hasher.update(right);

                let mut parent = [0u8; 32];
                parent.copy_from_slice(&hasher.finalize());
                next.push(parent);
            }

            level = next;
        }

        level[0]
    }
}

// 模拟的高维张量 (简化为 16 维向量用于快速测试)
type Tensor = [f32; 16];

// 全局语义摩擦力引力阈值
const EPSILON_THRESHOLD: f32 = 0.15;

/// 核心数学原语：计算意图张量 I 与能力张量 C 的余弦摩擦力
fn calculate_semantic_friction(intent: &Tensor, capability: &Tensor) -> f32 {
    let mut dot_product = 0.0;
    let mut norm_i = 0.0;
    let mut norm_c = 0.0;

    for i in 0..16 {
        dot_product += intent[i] * capability[i];
        norm_i += intent[i] * intent[i];
        norm_c += capability[i] * capability[i];
    }

    1.0 - (dot_product / (norm_i.sqrt() * norm_c.sqrt() + 1e-8))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn swarm_simulator_generates_merkle_root() {
    println!("===========================================================");
    println!("🌌 LIFE++ TESTNET: IGNITING SILICON SWARM (1 ORCHESTRATOR, 100 WORKERS)");
    println!("===========================================================\\n");

    let (gossip_tx, _) = broadcast::channel::<Tensor>(100);
    let (aggregator_tx, mut aggregator_rx) = mpsc::channel::<SimCognitiveTransaction>(1000);

    for worker_id in 0..100 {
        let mut gossip_receiver = gossip_tx.subscribe();
        let tx_to_aggregator = aggregator_tx.clone();

        tokio::spawn(async move {
            let capability: Tensor = {
                let mut rng = rand::thread_rng();
                let mut tensor = std::array::from_fn(|_| rng.gen_range(0.0..1.0));
                if worker_id % 20 == 0 {
                    tensor = [0.0; 16];
                    tensor[3] = rng.gen_range(0.85..1.0);
                    tensor[7] = rng.gen_range(0.75..1.0);
                }
                tensor
            };

            while let Ok(intent) = gossip_receiver.recv().await {
                let friction = calculate_semantic_friction(&intent, &capability);

                if friction <= EPSILON_THRESHOLD {
                    let delay_ms = {
                        let mut rng = rand::thread_rng();
                        rng.gen_range(10..50)
                    };
                    sleep(Duration::from_millis(delay_ms)).await;

                    let ctx = SimCognitiveTransaction {
                        agent_pubkey: format!("DID:AHIN:WORKER_{:03}", worker_id),
                        semantic_friction: friction,
                        settled_volume: 0.00025,
                        signature: vec![0u8; 64],
                    };

                    let _ = tx_to_aggregator.send(ctx).await;
                }
            }
        });
    }

    let orchestrator_task = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;

        let mut intent: Tensor = [0.0; 16];
        intent[3] = 0.9;
        intent[7] = 0.8;

        println!("📡 [ORCHESTRATOR] Broadcasting High-Dimensional Intent Tensor...");
        let _ = gossip_tx.send(intent);
    });

    let aggregator_task = tokio::spawn(async move {
        let mut batch_txs = Vec::new();
        let mut active_tree = ConcurrentMerkleTree::new();

        sleep(Duration::from_millis(500)).await;

        while let Ok(ctx) = aggregator_rx.try_recv() {
            println!(
                "✨ [COLLAPSE] Resonance with {} | Friction: {:.4} | Volume: {} USDC",
                ctx.agent_pubkey, ctx.semantic_friction, ctx.settled_volume
            );
            active_tree.insert(ctx.hash());
            batch_txs.push(ctx);
        }

        println!("\\n===========================================================");
        println!("🏛️ [ZK-AGGREGATOR] Network Epoch Concluded.");
        println!("📊 Total Successful Micro-payments (CTx): {}", batch_txs.len());
        println!("🔗 GENESIS MERKLE ROOT: {:?}", active_tree.root());
        println!("===========================================================");

        (batch_txs.len(), active_tree.root())
    });

    let _ = orchestrator_task.await;
    let (count, merkle_root) = aggregator_task.await.expect("aggregator task should complete");

    assert!(count > 0, "at least one worker should collapse into resonance");
    assert_ne!(merkle_root, [0u8; 32], "non-empty batch must produce non-zero root");
}

#[test]
fn semantic_friction_for_identical_tensors_is_near_zero() {
    let tensor = [0.1; 16];
    let friction = calculate_semantic_friction(&tensor, &tensor);
    assert!(friction.abs() < 1e-6);
}
