use crossbeam_channel::Receiver;
use methods::{NETTING_CIRCUIT_ELF, NETTING_CIRCUIT_ID};
use pocc_core::models::{CognitiveTransaction, StateTransitionBatch};
use risc0_zkvm::{default_prover, ExecutorEnv};
use std::time::Instant;

pub struct ZkAggregator {
    ctx_receiver: Receiver<CognitiveTransaction>,
    batch_size: usize,
}

impl ZkAggregator {
    /// 启动无限循环轧差折叠引擎
    pub fn run_folding_engine(&self, initial_root: [u8; 32]) {
        println!(
            "🌌 [AGGREGATOR] ZK-STARK Folding Engine ignited. Assimilating x402 micropayments..."
        );

        let mut active_root = initial_root;

        loop {
            let mut batch_txs = Vec::with_capacity(self.batch_size);

            // 极速收集预定数量的认知交易（例如 10,000 笔）
            for _ in 0..self.batch_size {
                if let Ok(ctx) = self.ctx_receiver.recv() {
                    batch_txs.push(ctx);
                }
            }

            println!(
                "📦 [BATCH] {} CTx collected. Commencing STARK proving...",
                self.batch_size
            );
            let start_time = Instant::now();

            let batch_payload = StateTransitionBatch {
                old_state_root: active_root,
                transactions: batch_txs,
            };

            // 1) 构建 ZKVM 内存映像并注入批处理账单数据
            let env = ExecutorEnv::builder()
                .write(&batch_payload)
                .unwrap()
                .build()
                .unwrap();

            // 2) 生成 STARK 证明
            let prover = default_prover();
            let prove_info = prover.prove(env, NETTING_CIRCUIT_ELF).unwrap();
            let receipt = prove_info.receipt;

            // 3) 提取并推进新状态根
            let (old_root, new_root, batch_root): ([u8; 32], [u8; 32], [u8; 32]) =
                receipt.journal.decode().unwrap();
            assert_eq!(old_root, active_root, "state root continuity violation");
            active_root = new_root;

            let elapsed = start_time.elapsed().as_secs_f32();
            println!(
                "✨ [STARK GENERATED] Massive tensor consensus collapsed into a single mathematical truth in {:.2}s.",
                elapsed
            );
            println!("🧾 Circuit ID: {:?}", NETTING_CIRCUIT_ID);
            println!("🧩 Batch Root: {:?}", batch_root);
            println!("🔗 New Planetary State Root: {:?}", new_root);

            // 4) 提交压缩后的证明到 L1
            // self.submit_proof_to_l1(receipt);
        }
    }
}
