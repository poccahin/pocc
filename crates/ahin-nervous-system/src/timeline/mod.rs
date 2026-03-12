use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TimelineError {
    #[error("Avalanche failure")]
    AvalancheFailure,
}

pub type CogHash = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CogNode {
    pub previous_hash: CogHash,
    pub agent_did: String,
    pub ctx_payload_hash: CogHash,
    pub zk_cog_proof_commitment: Vec<u8>,
    pub timestamp: i64,
}

impl CogNode {
    pub fn calculate_hash(&self) -> CogHash {
        let mut hasher = Sha256::new();
        hasher.update(&bincode::serialize(&self).unwrap());
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hasher.finalize());
        hash
    }
}

pub struct CognitiveHashTimeline {
    chain: Vec<(CogHash, CogNode)>,
}

impl CognitiveHashTimeline {
    pub fn new(genesis_node: CogNode) -> Self {
        let hash = genesis_node.calculate_hash();
        Self { chain: vec![(hash, genesis_node)] }
    }
    pub fn anchor_cognition(&mut self, node: CogNode) -> Result<CogHash, TimelineError> {
        if node.previous_hash != self.get_latest_hash() {
            return Err(TimelineError::AvalancheFailure);
        }
        let hash = node.calculate_hash();
        self.chain.push((hash, node));
        Ok(hash)
    }
    pub fn get_latest_hash(&self) -> CogHash {
        self.chain.last().expect("chain always has at least the genesis node").0
    }
}
