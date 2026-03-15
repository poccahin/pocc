#![no_main]

use pocc_core::crypto::verify_signature;
use pocc_core::merkle::ConcurrentMerkleTree;
use pocc_core::models::StateTransitionBatch;
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

pub fn main() {
    // 1. 从宿主机（链下聚合节点）读取大规模批处理数据
    // 包含：前序全局状态根、批量 CTx 明细
    let batch: StateTransitionBatch = env::read();

    let current_state_root = batch.old_state_root;
    let mut batch_tree = ConcurrentMerkleTree::new();

    // 严苛的全局语义摩擦力阈值
    const EPSILON_THRESHOLD: f32 = 0.05;

    // 2. 在 ZKVM 内验证每笔认知协作交易
    for ctx in batch.transactions.iter() {
        // 绝对规则 1：验证张量语义摩擦力
        if ctx.semantic_friction > EPSILON_THRESHOLD {
            panic!("[ZKVM FATAL] Cognitive Transaction rejected. Semantic friction exceeds Epsilon.");
        }

        // 绝对规则 2：验证 x402 签名与底层密码学主权
        let is_valid_sig = verify_signature(&ctx.agent_pubkey, &ctx.payload_hash(), &ctx.signature);

        if !is_valid_sig {
            panic!("[ZKVM FATAL] Cryptographic Heresy. Unauthorized entity detected in batch.");
        }

        // 绝对规则 3：热力学守恒验证
        if ctx.settled_volume <= 0.0 {
            panic!("[ZKVM FATAL] Zero-value void transaction detected. Sybil attempt blocked.");
        }

        batch_tree.insert(ctx.hash());
    }

    // 3. 生成批次根
    let batch_root = batch_tree.root();

    // 4. 合并到全局状态根
    let new_state_root = hash_combine(current_state_root, batch_root);

    // 5. 提交公开承诺（随证明上链）
    env::commit(&(batch.old_state_root, new_state_root, batch_root));
}

fn hash_combine(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    pocc_core::crypto::poseidon_hash(&[a, b].concat())
}
