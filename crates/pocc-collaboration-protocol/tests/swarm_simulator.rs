use std::sync::Arc;

use pocc_collaboration_protocol::merkle::ConcurrentMerkleTree;
use pocc_collaboration_protocol::{
    CognitiveBoundary, CognitiveTransaction, CtxComposer, SettlementInstruction,
};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};

// Simplified high-dimensional tensor (16-D vector for fast CI tests)
type Tensor = [f32; 16];

// Semantic-friction gate threshold
const EPSILON_THRESHOLD: f32 = 0.15;

fn calculate_semantic_friction(intent: &Tensor, capability: &Tensor) -> f32 {
    let mut dot_product = 0.0_f32;
    let mut norm_i = 0.0_f32;
    let mut norm_c = 0.0_f32;

    for i in 0..16 {
        dot_product += intent[i] * capability[i];
        norm_i += intent[i] * intent[i];
        norm_c += capability[i] * capability[i];
    }

    1.0 - (dot_product / (norm_i.sqrt() * norm_c.sqrt() + 1e-8))
}

/// Return a deterministic cohort of 100 worker capability tensors, all
/// collinear with the supplied orchestrator intent (scaled versions of it),
/// so the semantic-friction value is near-zero for every worker.
fn deterministic_cohort(intent: &Tensor) -> Vec<Tensor> {
    (0..100u32)
        .map(|i| {
            // Scale from 0.80 to ~1.00; collinear tensors have near-zero friction.
            let scale = 0.8 + (i as f32) * 0.002;
            std::array::from_fn(|j| intent[j] * scale)
        })
        .collect()
}

/// Validate that calculate_semantic_friction returns a value near zero
/// when the intent and capability tensors are identical.
#[test]
fn semantic_friction_for_identical_tensors_is_near_zero() {
    let tensor: Tensor = [0.5; 16];
    let friction = calculate_semantic_friction(&tensor, &tensor);
    assert!(
        friction < 1e-5,
        "expected near-zero friction for identical tensors, got {friction}"
    );
}

/// Simulate 1 Orchestrator + 100 Workers.  All workers are seeded with
/// tensors aligned to the orchestrator intent, so every worker passes the
/// semantic-friction gate and contributes a leaf to the Merkle tree.
/// The test asserts that the resulting Merkle root is non-zero.
///
/// Synchronisation is channel-based (no arbitrary sleeps):
///  • All 100 workers subscribe to the gossip channel synchronously before
///    the intent is broadcast, so no message is ever missed.
///  • The aggregator channel closes naturally once every worker task exits,
///    so the aggregator drains the channel without a fixed timeout.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn swarm_simulator_generates_merkle_root() {
    let (gossip_tx, _) = tokio::sync::broadcast::channel::<Tensor>(128);
    let (aggregator_tx, mut aggregator_rx) = mpsc::channel::<CognitiveTransaction>(1000);

    let intent: Tensor = {
        let mut t = [0.0_f32; 16];
        t[3] = 0.9;
        t[7] = 0.8;
        t
    };

    let cohort = deterministic_cohort(&intent);
    let generated = Arc::new(Mutex::new(0usize));

    // Subscribe every worker to the gossip channel synchronously in the main
    // task.  All subscriptions are registered before the first broadcast, so
    // no worker can miss the intent signal.
    for (worker_id, capability) in cohort.into_iter().enumerate() {
        let mut gossip_receiver = gossip_tx.subscribe();
        let tx_to_aggregator = aggregator_tx.clone();
        let generated_ref = Arc::clone(&generated);

        tokio::spawn(async move {
            while let Ok(broadcast_intent) = gossip_receiver.recv().await {
                let friction = calculate_semantic_friction(&broadcast_intent, &capability);
                if friction > EPSILON_THRESHOLD {
                    continue;
                }

                // Simulate brief compute time.
                sleep(Duration::from_millis(10)).await;

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

                // Increment before send so the count is visible to the
                // aggregator assertion regardless of scheduling order.
                *generated_ref.lock().await += 1;
                let _ = tx_to_aggregator.send(ctx).await;
            }
            // tx_to_aggregator is dropped here; when all 100 tasks finish the
            // aggregator channel closes automatically.
        });
    }

    // Drop the original aggregator sender so the channel is held open only
    // by the worker task clones.
    drop(aggregator_tx);

    // Broadcast the intent then drop gossip_tx.  Dropping the sole sender
    // closes the broadcast channel, which causes every worker's recv() to
    // return Err and exit its loop, eventually dropping all tx_to_aggregator
    // clones and closing the aggregator channel.
    let _ = gossip_tx.send(intent);
    drop(gossip_tx);

    // Drain the aggregator channel; recv() returns None only once all worker
    // tasks have finished and dropped their senders.
    let mut batch_txs = Vec::new();
    let mut active_tree = ConcurrentMerkleTree::new();
    while let Some(ctx) = aggregator_rx.recv().await {
        active_tree.insert(ctx.calculate_payload_hash());
        batch_txs.push(ctx);
    }

    let count = batch_txs.len();
    assert_eq!(
        count,
        *generated.lock().await,
        "aggregated count should match generated count"
    );
    assert!(count > 0, "at least one worker should participate");
    assert_ne!(active_tree.root(), [0u8; 32], "Merkle root must be non-zero");
}
