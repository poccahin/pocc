use pqcrypto_falcon::falcon1024::{verify_detached_signature, DetachedSignature, PublicKey};
use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct X402FinalState {
    pub channel_id: String,
    pub orchestrator_pubkey: Vec<u8>,      // Falcon-1024 public key (1793 bytes)
    pub final_nonce: u64,
    pub settled_balance: f64,
    pub cryptographic_signature: Vec<u8>,  // Falcon-1024 detached signature
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

        let pubkey = PublicKey::from_bytes(&tx.orchestrator_pubkey)
            .unwrap_or_else(|e| panic!(
                "Fatal: invalid Falcon-1024 orchestrator public key (expected {} bytes, got {}): {:?}",
                pqcrypto_falcon::falcon1024::public_key_bytes(),
                tx.orchestrator_pubkey.len(),
                e,
            ));
        let sig = DetachedSignature::from_bytes(&tx.cryptographic_signature)
            .unwrap_or_else(|e| panic!(
                "Fatal: invalid Falcon-1024 detached signature ({} bytes provided): {:?}",
                tx.cryptographic_signature.len(),
                e,
            ));
        verify_detached_signature(&sig, &state_hash, &pubkey)
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
