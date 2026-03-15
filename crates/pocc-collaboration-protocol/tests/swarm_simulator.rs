use std::sync::Arc;

use pocc_collaboration_protocol::merkle::ConcurrentMerkleTree;
use pocc_collaboration_protocol::{
    CognitiveBoundary, CognitiveTransaction, CtxComposer, SettlementInstruction,
};
use rand::Rng;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};

// 模拟的高维张量 (简化为 16 维向量用于快速测试)
type Tensor = [f32; 16];

// 全局语义摩擦力引力阈值
const EPSILON_THRESHOLD: f32 = 0.15;

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
async fn swarm_simulator_cluster_epoch() {
    println!("===========================================================");
    println!("🌌 LIFE++ TESTNET: IGNITING SILICON SWARM (1 ORCHESTRATOR, 100 WORKERS)");
    println!("===========================================================\n");

    let (gossip_tx, _gossip_rx) = tokio::sync::broadcast::channel::<Tensor>(100);
    let (aggregator_tx, mut aggregator_rx) = mpsc::channel::<CognitiveTransaction>(1000);

    let intent: Tensor = {
        let mut tensor = [0.0; 16];
        tensor[3] = 0.9;
        tensor[7] = 0.8;
        tensor
    };

    let generated = Arc::new(Mutex::new(0usize));

    for worker_id in 0..100 {
        let mut gossip_receiver = gossip_tx.subscribe();
        let tx_to_aggregator = aggregator_tx.clone();
        let generated = Arc::clone(&generated);

        tokio::spawn(async move {
            let capability: Tensor = {
                let mut rng = rand::thread_rng();
                std::array::from_fn(|_| rng.gen_range(0.0..1.0))
            };

            while let Ok(intent) = gossip_receiver.recv().await {
                let friction = calculate_semantic_friction(&intent, &capability);
                if friction > EPSILON_THRESHOLD {
                    continue;
                }

                let delay_ms = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(10..50)
                };
                sleep(Duration::from_millis(delay_ms)).await;

                let settlement = SettlementInstruction {
                    amount: 0.00025,
                    token_symbol: "USDC".to_string(),
                    buyer_signature: "simulated_signature".to_string(),
                };
                let boundary = CognitiveBoundary {
                    max_compute_units: 2_000,
                    max_time_ms: 100,
                    safety_clearance_level: 1,
                };
                let mut ctx = CtxComposer::draft_transaction(
                    "did:ahin:orchestrator:001",
                    &format!("DID:AHIN:WORKER_{worker_id:03}"),
                    "ACTIVE_HASH_INTENT",
                    boundary,
                    settlement,
                );
                ctx.is_executed = true;
                ctx.execution_output_hash = Some([worker_id as u8; 32]);

                let _ = tx_to_aggregator.send(ctx).await;
                *generated.lock().await += 1;
            }
        });
    }

    let orchestrator_task = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        println!("📡 [ORCHESTRATOR] Broadcasting High-Dimensional Intent Tensor...");
        let _ = gossip_tx.send(intent);
    });

    let aggregator_task = tokio::spawn(async move {
        let mut batch_txs = Vec::new();
        let mut active_tree = ConcurrentMerkleTree::new();

        sleep(Duration::from_millis(500)).await;

        while let Ok(ctx) = aggregator_rx.try_recv() {
            println!(
                "✨ [COLLAPSE] Resonance with {} | Friction: <= {:.2} | Volume: {} USDC",
                ctx.seller_did, EPSILON_THRESHOLD, ctx.settlement.amount
            );

            active_tree.insert(ctx.calculate_payload_hash());
            batch_txs.push(ctx);
        }

        println!("\n===========================================================");
        println!("🏛️ [ZK-AGGREGATOR] Network Epoch Concluded.");
        println!(
            "📊 Total Successful Micro-payments (CTx): {}",
            batch_txs.len()
        );
        println!("🔗 GENESIS MERKLE ROOT: {:?}", active_tree.root());
        println!("===========================================================");

        (batch_txs.len(), active_tree.root())
    });

    let _ = orchestrator_task.await;
    let (count, root) = aggregator_task
        .await
        .expect("aggregator task should complete");

    assert_eq!(count, *generated.lock().await);
    assert!(count <= 100);
    if count > 0 {
        assert_ne!(root, [0u8; 32]);
    }
}
