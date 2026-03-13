#![no_main]

use pocc_core::crypto::pqc_falcon::verify_detached;
use pocc_core::merkle::MicroMerkleTree;
use pocc_core::models::StateTransition;
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

pub fn main() {
    // 1. 从宿主机（链下聚合器）读取输入数据：前序状态根 + CTx 批次
    let transition: StateTransition = env::read();

    let mut batch_tree = MicroMerkleTree::new();

    // 2. 在 ZKVM 内验证每笔认知交易
    for ctx in transition.transactions.iter() {
        // 核心计算 1：验证抗量子签名（FALCON）
        let is_valid =
            verify_detached(&ctx.signature, &ctx.payload_hash(), &ctx.payer_pubkey).is_ok();

        if !is_valid {
            panic!("[ZKVM FATAL] Invalid CTx signature detected in batch");
        }

        // 核心计算 2：守恒约束（金额必须为正）
        if ctx.amount <= 0 {
            panic!("[ZKVM FATAL] Invalid CTx amount, expected positive value");
        }

        batch_tree.insert(ctx.hash());
    }

    // 3. 生成批次根
    let batch_root = batch_tree.root();

    // 4. 合并到全局状态根
    let new_state_root = hash_combine(transition.old_state_root, batch_root);

    // 5. 提交公开承诺（随证明上链）
    env::commit(&(transition.old_state_root, new_state_root, batch_root));
}

fn hash_combine(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    pocc_core::crypto::sha3_hash(&[a, b].concat())
}
