use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct X402FinalState {
    pub channel_id: String,
    pub orchestrator_pubkey: [u8; 32],
    pub final_nonce: u64,
    pub settled_balance: f64,
    pub cryptographic_signature: [u8; 64],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RollupBatch {
    pub previous_state_root: [u8; 32],
    pub transactions: Vec<X402FinalState>,
}

fn main() {
    let batch: RollupBatch = env::read();

    let mut current_root = batch.previous_state_root;
    let mut total_lifeplus_settled = 0.0_f64;

    for tx in &batch.transactions {
        let mut hasher = Sha256::new();
        hasher.update(tx.channel_id.as_bytes());
        hasher.update(tx.final_nonce.to_be_bytes());
        hasher.update(tx.settled_balance.to_be_bytes());
        let state_hash: [u8; 32] = hasher.finalize().into();

        let pubkey = VerifyingKey::from_bytes(&tx.orchestrator_pubkey)
            .expect("Fatal: invalid orchestrator public key");
        let sig = Signature::from_bytes(&tx.cryptographic_signature);
        pubkey
            .verify(&state_hash, &sig)
            .expect("Fatal: cryptographic signature mismatch in channel");

        total_lifeplus_settled += tx.settled_balance;

        let mut root_hasher = Sha256::new();
        root_hasher.update(current_root);
        root_hasher.update(state_hash);
        current_root = root_hasher.finalize().into();
    }

    env::commit(&(
        batch.previous_state_root,
        current_root,
        total_lifeplus_settled,
    ));
}
