use pocc_collaboration_protocol::ctx_composer::CognitiveTransaction;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub struct DailyNettingProcessor {
    pub token_symbol: String,
    pending_transactions: Vec<CognitiveTransaction>,
}

/// 某个智能体在轧账周期内的净头寸。
/// `net_amount > 0` 表示该 DID 是净债权方（应收），`< 0` 表示净债务方（应付）。
#[derive(Debug, Clone)]
pub struct NetPosition {
    pub did: String,
    pub net_amount: f64,
}

/// 轧账批次：包含所有净头寸及批次内容哈希，可直接上链作为结算凭证。
#[derive(Debug)]
pub struct SettlementBatch {
    pub epoch: u64,
    pub net_positions: Vec<NetPosition>,
    /// SHA-256 over all net positions, deterministic serialisation.
    pub batch_hash: [u8; 32],
}

impl DailyNettingProcessor {
    pub fn new(token_symbol: &str) -> Self {
        Self {
            token_symbol: token_symbol.to_string(),
            pending_transactions: Vec::new(),
        }
    }

    /// 将一笔认知交易放入待处理队列。
    pub fn ingest(&mut self, tx: CognitiveTransaction) {
        self.pending_transactions.push(tx);
    }

    /// 执行多边净额轧账（Multilateral Netting），返回结算批次并清空队列。
    ///
    /// 算法：
    /// 1. 遍历所有 `CognitiveTransaction`，以 `buyer_did → seller_did` 方向累加金额。
    /// 2. 对每个 DID 计算 `净头寸 = Σ 应收 − Σ 应付`。
    /// 3. 将净头寸列表按 DID 字典序排列后做 SHA-256，作为批次哈希。
    pub fn clear(&mut self, epoch: u64) -> SettlementBatch {
        let mut ledger: HashMap<String, f64> = HashMap::new();

        for tx in self.pending_transactions.drain(..) {
            let amount = tx.settlement.amount;
            *ledger.entry(tx.seller_did).or_insert(0.0) += amount;
            *ledger.entry(tx.buyer_did).or_insert(0.0) -= amount;
        }

        let mut net_positions: Vec<NetPosition> = ledger
            .into_iter()
            .map(|(did, net_amount)| NetPosition { did, net_amount })
            .collect();

        net_positions.sort_by(|a, b| a.did.cmp(&b.did));

        let batch_hash = Self::hash_positions(epoch, &net_positions);

        SettlementBatch { epoch, net_positions, batch_hash }
    }

    fn hash_positions(epoch: u64, positions: &[NetPosition]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(epoch.to_be_bytes());
        for pos in positions {
            hasher.update(pos.did.as_bytes());
            hasher.update(pos.net_amount.to_be_bytes());
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocc_collaboration_protocol::ctx_composer::{
        CognitiveBoundary, CognitiveTransaction, SettlementInstruction,
    };

    fn make_tx(buyer: &str, seller: &str, amount: f64) -> CognitiveTransaction {
        CognitiveTransaction {
            ctx_id: format!("{buyer}->{seller}"),
            buyer_did: buyer.to_string(),
            seller_did: seller.to_string(),
            intent_declaration: "test".to_string(),
            l0_kinetic_command_hash: None,
            boundary: CognitiveBoundary {
                max_compute_units: 1000,
                max_time_ms: 60_000,
                safety_clearance_level: 1,
            },
            settlement: SettlementInstruction {
                amount,
                token_symbol: "LIFE++".to_string(),
                buyer_signature: "sig_test".to_string(),
            },
            is_executed: false,
            execution_output_hash: None,
            zk_proof_commitment: None,
            timestamp: 0,
        }
    }

    #[test]
    fn net_positions_sum_to_zero() {
        let mut processor = DailyNettingProcessor::new("LIFE++");
        processor.ingest(make_tx("A", "B", 10.0));
        processor.ingest(make_tx("C", "A", 4.0));
        processor.ingest(make_tx("B", "C", 6.0));
        let batch = processor.clear(1);
        let total: f64 = batch.net_positions.iter().map(|p| p.net_amount).sum();
        assert!(total.abs() < 1e-9, "net positions must sum to zero, got {total}");
    }

    #[test]
    fn clear_empties_pending_queue() {
        let mut processor = DailyNettingProcessor::new("LIFE++");
        processor.ingest(make_tx("X", "Y", 5.0));
        processor.clear(1);
        let batch2 = processor.clear(2);
        assert!(batch2.net_positions.is_empty());
    }

    #[test]
    fn batch_hash_is_deterministic() {
        let mut p1 = DailyNettingProcessor::new("LIFE++");
        p1.ingest(make_tx("A", "B", 7.0));
        let b1 = p1.clear(42);

        let mut p2 = DailyNettingProcessor::new("LIFE++");
        p2.ingest(make_tx("A", "B", 7.0));
        let b2 = p2.clear(42);

        assert_eq!(b1.batch_hash, b2.batch_hash);
    }
}
