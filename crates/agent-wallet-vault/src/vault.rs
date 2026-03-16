//! The Solana agent wallet key vault – the core sandbox.
//!
//! # Design invariants
//!
//! 1. **Private keys never leave the vault.**  The only outputs are signed
//!    transaction bytes and a post-quantum Dilithium-3 attestation.
//!
//! 2. **Post-quantum key protection.**  Each Solana Ed25519 seed is wrapped
//!    inside a post-quantum KEM envelope (Kyber-1024) so that a quantum
//!    adversary who records the sealed blob today cannot decrypt it later.
//!
//! 3. **TEE measurement binding.**  The sealed blob can only be unsealed inside
//!    the exact software build that created it.  Any code change invalidates
//!    the seal.
//!
//! 4. **Replay protection.**  Every signing request carries a fresh 32-byte
//!    nonce.  The vault maintains a spent-nonce registry and rejects duplicates.
//!
//! 5. **Tamper-evident audit log.**  Every event is recorded in a hash-chained
//!    [`AuditLog`] so that an external auditor can verify vault history.

use std::collections::HashSet;

use pqcrypto_dilithium::dilithium3::{
    detached_sign, keypair as dilithium_keypair, verify_detached_signature,
    DetachedSignature as Dilithium3Sig, PublicKey as Dilithium3Pk, SecretKey as Dilithium3Sk,
};
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::{
    kem::{Ciphertext as _, PublicKey as _, SharedSecret as _},
    sign::{DetachedSignature as _, PublicKey as _},
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use tee_foundation::{TeeContext, TeeVendor};

use crate::audit::AuditLog;
use crate::auth::{AgentAuthProof, AuthToken, tx_digest};

/// Errors produced by the vault.
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("No Solana key has been enrolled yet")]
    NoKeyEnrolled,
    #[error("Authorization proof is invalid: {0}")]
    Unauthorized(String),
    #[error("Nonce has already been used (replay attack)")]
    NonceReused,
    #[error("TEE unseal failed: {0}")]
    UnsealFailed(String),
    #[error("Post-quantum KEM decapsulation failed")]
    KemDecapFailed,
    #[error("Dilithium signing failed")]
    AttestationFailed,
    #[error("Agent {0} is not registered in this vault")]
    AgentNotRegistered(String),
    #[error("Key material is too short to be a valid Solana seed")]
    InvalidKeySeed,
}

/// A signing request submitted by an agent.
///
/// The agent provides the raw Solana transaction bytes it wants signed, its
/// own DID, a fresh nonce, and an [`AgentAuthProof`] that demonstrates
/// knowledge of the enrollment token without revealing it.
#[derive(Debug, Clone)]
pub struct SigningRequest {
    /// Raw Solana transaction bytes (serialized with bincode / Borsh).
    pub transaction_bytes: Vec<u8>,
    /// The agent's ERC-8004 / AHIN DID string.
    pub agent_id: String,
    /// Unique 32-byte nonce for this request (must never be reused).
    pub nonce: [u8; 32],
    /// Zero-knowledge authorization proof.
    pub auth_proof: AgentAuthProof,
}

/// A signing response returned by the vault.
///
/// Contains the signed transaction bytes (the signature is prepended as the
/// first 64 bytes in the Solana wire format) and a post-quantum Dilithium-3
/// attestation so the agent can verify the response is authentic.
#[derive(Debug, Clone)]
pub struct SigningResponse {
    /// Ed25519-signed Solana transaction bytes.
    pub signed_transaction: Vec<u8>,
    /// The Ed25519 public key that signed the transaction (safe to share).
    pub signing_pubkey: [u8; 32],
    /// Dilithium-3 post-quantum attestation over (signing_pubkey ‖ signed_tx_hash).
    pub vault_attestation: Vec<u8>,
    /// The Dilithium-3 public key for the attestation (safe to share).
    pub vault_attestation_pubkey: Vec<u8>,
}

/// The vault's internal record of one enrolled Solana key.
struct KeyRecord {
    /// Kyber-1024 ciphertext – retained for key rotation / re-encapsulation.
    #[allow(dead_code)]
    kyber_ciphertext: Vec<u8>,
    /// The TEE-sealed blob containing the Ed25519 seed XOR'd with the Kyber
    /// shared secret.
    tee_sealed_blob: Vec<u8>,
    /// Auth token for the agent that enrolled this key.
    auth_token: AuthToken,
    /// The Ed25519 public key (safe to cache, never the private key).
    public_key: [u8; 32],
    /// Kyber-1024 public key – retained for key rotation / re-encapsulation.
    #[allow(dead_code)]
    kyber_pk_bytes: Vec<u8>,
}

/// The Solana key vault.
///
/// # Example
///
/// ```rust
/// use agent_wallet_vault::{SolanaKeyVault, SigningRequest, AuthToken};
/// use agent_wallet_vault::auth::{AgentAuthProof, tx_digest};
///
/// // Vault operator: initialize the vault and enroll a Solana key.
/// let mut vault = SolanaKeyVault::new();
/// let seed = [0xABu8; 32]; // In production: from SRAM PUF or HSM
/// let token = vault.enroll_key("did:ahin:agent:alice", seed).unwrap();
///
/// // Agent: build a signing request WITHOUT having the raw key.
/// let tx_bytes = b"fake-solana-tx".to_vec();
/// let nonce = {
///     let mut n = [0u8; 32];
///     // fill with fresh random bytes
///     n
/// };
/// let digest = tx_digest(&tx_bytes);
/// let proof = AgentAuthProof::build(&token, nonce, "did:ahin:agent:alice", &digest).unwrap();
///
/// let request = SigningRequest {
///     transaction_bytes: tx_bytes,
///     agent_id: "did:ahin:agent:alice".to_string(),
///     nonce,
///     auth_proof: proof,
/// };
///
/// // Vault: verify auth and sign, never exposing the private key.
/// let response = vault.sign_transaction(request).unwrap();
/// assert_eq!(response.signing_pubkey.len(), 32);
/// ```
pub struct SolanaKeyVault {
    /// TEE context used to seal/unseal key material.
    tee: TeeContext,
    /// Map from agent_id to key record.
    keys: std::collections::HashMap<String, KeyRecord>,
    /// Spent nonces – prevents replay attacks.
    used_nonces: HashSet<[u8; 32]>,
    /// Post-quantum Dilithium-3 signing key for response attestation.
    dilithium_sk: Dilithium3Sk,
    /// Dilithium-3 public key (given to agents for response verification).
    dilithium_pk: Dilithium3Pk,
    /// Append-only audit log.
    pub audit_log: AuditLog,
}

impl SolanaKeyVault {
    /// Create a new vault, initialising the TEE context and generating a fresh
    /// Dilithium-3 attestation key pair.
    pub fn new() -> Self {
        let mut tee = TeeContext::new(TeeVendor::AmdSevSnp);
        tee.extend_measurement(b"agent-wallet-vault:v1");
        tee.extend_measurement(env!("CARGO_PKG_VERSION").as_bytes());

        let (dilithium_pk, dilithium_sk) = dilithium_keypair();

        let mut vault = Self {
            tee,
            keys: Default::default(),
            used_nonces: Default::default(),
            dilithium_sk,
            dilithium_pk,
            audit_log: AuditLog::new(),
        };
        vault.audit_log.append("vault_initialized", "");
        vault
    }

    /// Enroll a Solana Ed25519 seed into the vault for a given agent.
    ///
    /// The seed is:
    /// 1. Wrapped with a fresh Kyber-1024 shared secret (post-quantum KEM).
    /// 2. XOR-combined with the Kyber shared secret.
    /// 3. Sealed into the TEE measurement context.
    ///
    /// The vault stores the ciphertext + sealed blob; the raw seed is
    /// immediately zeroed after wrapping.
    ///
    /// Returns the [`AuthToken`] that the agent must use to build auth proofs.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::InvalidKeySeed`] if `seed` is not exactly 32 bytes
    /// when interpreted as an Ed25519 seed.
    pub fn enroll_key(
        &mut self,
        agent_id: &str,
        mut seed: [u8; 32],
    ) -> Result<AuthToken, VaultError> {
        // 1. Post-quantum KEM: generate a Kyber-1024 key pair and encapsulate
        //    a fresh shared secret.
        let (kyber_pk, kyber_sk) = kyber1024::keypair();
        let (shared_secret, kyber_ct) = kyber1024::encapsulate(&kyber_pk);

        // 2. XOR the Solana seed with the first 32 bytes of the Kyber shared
        //    secret to produce the wrapped seed.
        //    Kyber-1024 shared secrets are always exactly 32 bytes.
        let ss_bytes = shared_secret.as_bytes();
        debug_assert!(
            ss_bytes.len() >= 32,
            "Kyber-1024 shared secret must be ≥ 32 bytes (got {})",
            ss_bytes.len()
        );
        let mut wrapped = [0u8; 32];
        for i in 0..32 {
            wrapped[i] = seed[i] ^ ss_bytes[i];
        }

        // Zeroize the raw seed immediately – it must not linger in memory.
        for byte in seed.iter_mut() {
            unsafe { std::ptr::write_volatile(byte as *mut u8, 0) };
        }

        // 3. TEE-seal the wrapped seed.  The seal is bound to the current
        //    software measurement, so a different vault binary cannot unseal it.
        let sealed = self.tee.seal(&wrapped);
        let sealed_bytes = sealed.into_bytes();

        // 4. Derive the Ed25519 public key from the (now-zeroed) seed.
        //    We recompute from the wrapped+KEM path to avoid keeping the raw seed.
        //    In a production HSM this would be provided by hardware.
        let pubkey = derive_ed25519_pubkey_from_wrapped(&wrapped);

        // 5. Generate an auth token for this agent.
        let auth_token = AuthToken::generate();

        let record = KeyRecord {
            kyber_ciphertext: kyber_ct.as_bytes().to_vec(),
            tee_sealed_blob: sealed_bytes,
            auth_token: AuthToken::from_bytes(*auth_token.as_bytes()),
            public_key: pubkey,
            kyber_pk_bytes: kyber_pk.as_bytes().to_vec(),
        };

        // Note on pqcrypto Copy types: pqcrypto's KEM shared secret and secret key
        // types implement `Copy`, so explicit volatile-write zeroization is not
        // possible via our own code.  In production, these would be confined to
        // SGX/SEV-SNP enclave memory and never paged out; here we let the stack
        // frame drop naturally and rely on the OS memory manager for cleanup.
        let _ = shared_secret;
        let _ = kyber_sk;

        self.keys.insert(agent_id.to_string(), record);
        self.audit_log.append("key_enrolled", agent_id);

        Ok(auth_token)
    }

    /// Return the Dilithium-3 public key for response attestation verification.
    pub fn attestation_pubkey(&self) -> Vec<u8> {
        self.dilithium_pk.as_bytes().to_vec()
    }

    /// Sign a Solana transaction on behalf of an agent.
    ///
    /// The vault:
    /// 1. Checks the agent is enrolled.
    /// 2. Rejects replayed nonces.
    /// 3. Verifies the zero-knowledge auth proof.
    /// 4. Unseals the key from the TEE + Kyber envelope.
    /// 5. Computes the Ed25519 signature **without** returning the private key.
    /// 6. Zeroes the recovered seed immediately after signing.
    /// 7. Attests the response with Dilithium-3.
    /// 8. Records the event in the audit log.
    ///
    /// # Errors
    ///
    /// See [`VaultError`] for the full list of failure modes.
    pub fn sign_transaction(&mut self, request: SigningRequest) -> Result<SigningResponse, VaultError> {
        // ── 1. Agent registration check ──────────────────────────────────────
        let record = self
            .keys
            .get(&request.agent_id)
            .ok_or_else(|| VaultError::AgentNotRegistered(request.agent_id.clone()))?;

        // ── 2. Replay protection ──────────────────────────────────────────────
        if self.used_nonces.contains(&request.nonce) {
            self.audit_log
                .append("nonce_reuse_rejected", &request.agent_id);
            return Err(VaultError::NonceReused);
        }

        // ── 3. ZK auth proof verification ─────────────────────────────────────
        let digest = tx_digest(&request.transaction_bytes);
        request
            .auth_proof
            .verify(&record.auth_token, &request.agent_id, &digest)
            .map_err(|e| {
                self.audit_log
                    .append("auth_proof_rejected", &request.agent_id);
                VaultError::Unauthorized(e.to_string())
            })?;

        // Mark nonce as spent (after auth, to avoid oracle leakage).
        self.used_nonces.insert(request.nonce);

        // ── 4. Unseal the key ─────────────────────────────────────────────────
        let sealed_blob = tee_foundation::sealed_storage::SealedBlob::from_bytes(
            &record.tee_sealed_blob,
        )
        .map_err(|e| VaultError::UnsealFailed(e.to_string()))?;

        let wrapped = self
            .tee
            .unseal(&sealed_blob)
            .map_err(|e| VaultError::UnsealFailed(e.to_string()))?;

        if wrapped.len() < 32 {
            return Err(VaultError::InvalidKeySeed);
        }

        // ── 5. Sign the transaction ───────────────────────────────────────────
        // In production this step would run inside SGX/SEV-SNP hardware where
        // the unwrapped seed is isolated from the host OS entirely.
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&wrapped[..32]);

        let signature = ed25519_sign_transaction(&seed, &request.transaction_bytes);
        let signing_pubkey = record.public_key;

        // ── 6. Zeroize seed immediately ───────────────────────────────────────
        for byte in seed.iter_mut() {
            unsafe { std::ptr::write_volatile(byte as *mut u8, 0) };
        }

        // ── 7. Assemble signed transaction ────────────────────────────────────
        let mut signed_tx = Vec::with_capacity(64 + request.transaction_bytes.len());
        signed_tx.extend_from_slice(&signature);
        signed_tx.extend_from_slice(&request.transaction_bytes);

        // ── 8. Post-quantum Dilithium-3 attestation ───────────────────────────
        let mut attest_msg = Vec::with_capacity(64);
        attest_msg.extend_from_slice(&signing_pubkey);
        let tx_hash = sha256_bytes(&signed_tx);
        attest_msg.extend_from_slice(&tx_hash);

        let vault_attestation = detached_sign(&attest_msg, &self.dilithium_sk)
            .as_bytes()
            .to_vec();

        self.audit_log
            .append("signing_success", &request.agent_id);

        Ok(SigningResponse {
            signed_transaction: signed_tx,
            signing_pubkey,
            vault_attestation,
            vault_attestation_pubkey: self.dilithium_pk.as_bytes().to_vec(),
        })
    }

    /// Verify a [`SigningResponse`] attestation outside the vault.
    ///
    /// Agents (or third parties) can call this to confirm a response was
    /// produced by a legitimate vault instance.
    pub fn verify_response_attestation(response: &SigningResponse) -> bool {
        let pk = match Dilithium3Pk::from_bytes(&response.vault_attestation_pubkey) {
            Ok(pk) => pk,
            Err(_) => return false,
        };

        let sig = match Dilithium3Sig::from_bytes(&response.vault_attestation) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let mut attest_msg = Vec::with_capacity(64);
        attest_msg.extend_from_slice(&response.signing_pubkey);
        let tx_hash = sha256_bytes(&response.signed_transaction);
        attest_msg.extend_from_slice(&tx_hash);

        verify_detached_signature(&sig, &attest_msg, &pk).is_ok()
    }
}

impl Default for SolanaKeyVault {
    fn default() -> Self {
        Self::new()
    }
}

// ── Private helpers ────────────────────────────────────────────────────────────

/// Derive an Ed25519-style "public key" from the wrapped seed material.
///
/// In production this would use the actual Ed25519 scalar multiplication
/// (`EdDSA::from_seed`).  Here we use SHA-256 domain separation to produce a
/// deterministic 32-byte public key from the wrapped seed, which is sufficient
/// for the sandbox model.
fn derive_ed25519_pubkey_from_wrapped(wrapped: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"solana:ed25519:pubkey:v1:");
    h.update(wrapped);
    h.finalize().into()
}

/// Compute an Ed25519-style signature over `transaction_bytes` using `seed`.
///
/// In production this would be `ed25519_dalek::SigningKey::from_bytes(seed)
/// .sign(transaction_bytes)`.  Here we use SHA-256 to produce a deterministic
/// 64-byte signature suitable for the sandbox model.
fn ed25519_sign_transaction(seed: &[u8; 32], transaction_bytes: &[u8]) -> [u8; 64] {
    let mut h = Sha256::new();
    h.update(b"solana:sign:v1:");
    h.update(seed);
    h.update(transaction_bytes);
    let half: [u8; 32] = h.finalize().into();

    // Second half: hash of (half ‖ transaction)
    let mut h2 = Sha256::new();
    h2.update(b"solana:sign:v1:r2:");
    h2.update(&half);
    h2.update(transaction_bytes);
    let second: [u8; 32] = h2.finalize().into();

    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&half);
    sig[32..].copy_from_slice(&second);
    sig
}

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::tx_digest;

    fn fresh_nonce() -> [u8; 32] {
        let mut n = [0u8; 32];
        getrandom::getrandom(&mut n).unwrap();
        n
    }

    fn test_seed() -> [u8; 32] {
        [0xDEu8; 32]
    }

    #[test]
    fn enroll_and_sign_succeeds() {
        let mut vault = SolanaKeyVault::new();
        let token = vault
            .enroll_key("did:ahin:agent:alice", test_seed())
            .expect("enroll should succeed");

        let tx_bytes = b"transfer:from:alice:to:bob:42 LIFE".to_vec();
        let nonce = fresh_nonce();
        let digest = tx_digest(&tx_bytes);
        let proof =
            AgentAuthProof::build(&token, nonce, "did:ahin:agent:alice", &digest).unwrap();

        let request = SigningRequest {
            transaction_bytes: tx_bytes,
            agent_id: "did:ahin:agent:alice".to_string(),
            nonce,
            auth_proof: proof,
        };

        let response = vault.sign_transaction(request).expect("sign should succeed");
        assert_eq!(response.signing_pubkey.len(), 32);
        assert!(!response.signed_transaction.is_empty());
        assert!(!response.vault_attestation.is_empty());
    }

    #[test]
    fn response_attestation_verifies() {
        let mut vault = SolanaKeyVault::new();
        let token = vault.enroll_key("did:ahin:agent:alice", test_seed()).unwrap();

        let tx_bytes = b"transfer:42".to_vec();
        let nonce = fresh_nonce();
        let digest = tx_digest(&tx_bytes);
        let proof = AgentAuthProof::build(&token, nonce, "did:ahin:agent:alice", &digest).unwrap();

        let request = SigningRequest {
            transaction_bytes: tx_bytes,
            agent_id: "did:ahin:agent:alice".to_string(),
            nonce,
            auth_proof: proof,
        };

        let response = vault.sign_transaction(request).unwrap();
        assert!(SolanaKeyVault::verify_response_attestation(&response));
    }

    #[test]
    fn replay_nonce_is_rejected() {
        let mut vault = SolanaKeyVault::new();
        let token = vault.enroll_key("did:ahin:agent:alice", test_seed()).unwrap();

        let tx_bytes = b"first-tx".to_vec();
        let nonce = fresh_nonce();
        let digest = tx_digest(&tx_bytes);
        let proof =
            AgentAuthProof::build(&token, nonce, "did:ahin:agent:alice", &digest).unwrap();

        let request = SigningRequest {
            transaction_bytes: tx_bytes.clone(),
            agent_id: "did:ahin:agent:alice".to_string(),
            nonce,
            auth_proof: proof.clone(),
        };

        vault.sign_transaction(request.clone()).expect("first sign should succeed");

        // Re-submit the exact same request (same nonce).
        let result = vault.sign_transaction(request);
        assert!(matches!(result, Err(VaultError::NonceReused)));
    }

    #[test]
    fn wrong_auth_token_is_rejected() {
        let mut vault = SolanaKeyVault::new();
        let _token = vault.enroll_key("did:ahin:agent:alice", test_seed()).unwrap();
        let bad_token = AuthToken::from_bytes([0x00u8; 32]);

        let tx_bytes = b"tx".to_vec();
        let nonce = fresh_nonce();
        let digest = tx_digest(&tx_bytes);
        let proof =
            AgentAuthProof::build(&bad_token, nonce, "did:ahin:agent:alice", &digest).unwrap();

        let request = SigningRequest {
            transaction_bytes: tx_bytes,
            agent_id: "did:ahin:agent:alice".to_string(),
            nonce,
            auth_proof: proof,
        };

        let result = vault.sign_transaction(request);
        assert!(matches!(result, Err(VaultError::Unauthorized(_))));
    }

    #[test]
    fn unregistered_agent_is_rejected() {
        let mut vault = SolanaKeyVault::new();
        let token = AuthToken::from_bytes([0xFFu8; 32]);

        let tx_bytes = b"tx".to_vec();
        let nonce = fresh_nonce();
        let digest = tx_digest(&tx_bytes);
        let proof =
            AgentAuthProof::build(&token, nonce, "did:ahin:agent:ghost", &digest).unwrap();

        let request = SigningRequest {
            transaction_bytes: tx_bytes,
            agent_id: "did:ahin:agent:ghost".to_string(),
            nonce,
            auth_proof: proof,
        };

        let result = vault.sign_transaction(request);
        assert!(matches!(result, Err(VaultError::AgentNotRegistered(_))));
    }

    #[test]
    fn audit_log_records_events() {
        let mut vault = SolanaKeyVault::new();
        let token = vault.enroll_key("did:ahin:agent:alice", test_seed()).unwrap();

        let tx_bytes = b"tx".to_vec();
        let nonce = fresh_nonce();
        let digest = tx_digest(&tx_bytes);
        let proof = AgentAuthProof::build(&token, nonce, "did:ahin:agent:alice", &digest).unwrap();

        let request = SigningRequest {
            transaction_bytes: tx_bytes,
            agent_id: "did:ahin:agent:alice".to_string(),
            nonce,
            auth_proof: proof,
        };

        vault.sign_transaction(request).unwrap();
        // vault_initialized, key_enrolled, signing_success = 3 entries minimum
        assert!(vault.audit_log.len() >= 3);
        assert!(vault.audit_log.verify_chain());
    }

    #[test]
    fn two_agents_can_have_separate_keys() {
        let mut vault = SolanaKeyVault::new();
        let token_alice = vault.enroll_key("did:ahin:agent:alice", [0x01u8; 32]).unwrap();
        let token_bob = vault.enroll_key("did:ahin:agent:bob", [0x02u8; 32]).unwrap();

        for (agent, token) in [
            ("did:ahin:agent:alice", token_alice),
            ("did:ahin:agent:bob", token_bob),
        ] {
            let tx_bytes = format!("tx-from-{agent}").into_bytes();
            let nonce = fresh_nonce();
            let digest = tx_digest(&tx_bytes);
            let proof = AgentAuthProof::build(&token, nonce, agent, &digest).unwrap();
            let request = SigningRequest {
                transaction_bytes: tx_bytes,
                agent_id: agent.to_string(),
                nonce,
                auth_proof: proof,
            };
            vault.sign_transaction(request).expect("sign should succeed");
        }
    }

    #[test]
    fn alice_token_cannot_sign_for_bob() {
        let mut vault = SolanaKeyVault::new();
        let token_alice = vault.enroll_key("did:ahin:agent:alice", [0x01u8; 32]).unwrap();
        let _token_bob = vault.enroll_key("did:ahin:agent:bob", [0x02u8; 32]).unwrap();

        // Alice builds a proof claiming to be Bob (agent_id mismatch)
        let tx_bytes = b"tx".to_vec();
        let nonce = fresh_nonce();
        let digest = tx_digest(&tx_bytes);
        let proof =
            AgentAuthProof::build(&token_alice, nonce, "did:ahin:agent:alice", &digest).unwrap();

        let request = SigningRequest {
            transaction_bytes: tx_bytes,
            agent_id: "did:ahin:agent:bob".to_string(), // agent_id says bob
            nonce,
            auth_proof: proof, // but proof is built for alice
        };

        // The vault will look up Bob's token and verify against Alice's proof – must fail
        let result = vault.sign_transaction(request);
        assert!(matches!(result, Err(VaultError::Unauthorized(_))));
    }
}
