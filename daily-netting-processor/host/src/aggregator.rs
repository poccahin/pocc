use crossbeam_channel::Receiver;
use methods::{NETTING_CIRCUIT_ELF, NETTING_CIRCUIT_ID};
use pocc_core::models::{CognitiveTransaction, StateTransition};
use risc0_zkvm::{default_prover, ExecutorEnv};

pub struct ZkAggregator {
    ctx_receiver: Receiver<CognitiveTransaction>,
    batch_size: usize,
}

impl ZkAggregator {
    /// 启动无限循环轧差折叠引擎
    pub fn run_folding_loop(&self, current_root: [u8; 32]) {
        println!(
            "🌌 [AGGREGATOR] ZK-STARK Folding Engine ignited. Listening for x402 micropayments..."
        );

        let mut active_root = current_root;

        loop {
            let mut batch = Vec::with_capacity(self.batch_size);

            // 收集 N 笔认知交易组成批次
            for _ in 0..self.batch_size {
                if let Ok(ctx) = self.ctx_receiver.recv() {
                    batch.push(ctx);
                }
            }

            println!(
                "📦 [BATCH] {} CTx collected. Commencing STARK proving...",
                batch.len()
            );
            let start_time = std::time::Instant::now();

            let transition = StateTransition {
                old_state_root: active_root,
                transactions: batch,
            };

            // 1) 构建 ZKVM 执行环境
            let env = ExecutorEnv::builder()
                .write(&transition)
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
                "✨ [STARK GENERATED] {} txs collapsed into one proof in {:.2}s.",
                self.batch_size, elapsed
            );
            println!("🧾 Circuit ID: {:?}", NETTING_CIRCUIT_ID);
            println!("🧩 Batch Root: {:?}", batch_root);
            println!("🔗 New Global Root: {:?}", new_root);

            // 4) 提交压缩后的证明到 L1
            // self.submit_proof_to_l1(receipt);
        }
    }
}
