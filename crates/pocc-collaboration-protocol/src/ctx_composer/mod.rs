use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use ahin_nervous_system::CogHash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveBoundary { pub max_compute_units: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementInstruction { pub amount: f64, pub token_symbol: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveTransaction {
    pub ctx_id: String,
    pub buyer_did: String,
    pub seller_did: String,
    pub intent_declaration: String,
    pub boundary: CognitiveBoundary,
    pub settlement: SettlementInstruction,
    pub is_executed: bool,
}
impl CognitiveTransaction {
    pub fn calculate_payload_hash(&self) -> CogHash {
        let mut hasher = Sha256::new();
        hasher.update(self.ctx_id.as_bytes());
        hasher.update(self.buyer_did.as_bytes());
        hasher.update(self.seller_did.as_bytes());
        hasher.update(self.intent_declaration.as_bytes());
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hasher.finalize());
        hash
    }
}
