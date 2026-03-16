//! Agent authorization tokens and HMAC commitment proofs.
//!
//! An [`AuthToken`] is a 32-byte secret shared between the vault operator and
//! a specific agent at enrollment time.  The agent never sends the token in the
//! clear; instead it sends an [`AgentAuthProof`] – an HMAC-SHA256 commitment
//! that the vault can verify without the token ever crossing the isolation
//! boundary.

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::zk_commitment::ZkCommitmentScheme;

/// Errors produced during authorization.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Commitment construction failed: {0}")]
    CommitmentBuild(String),
    #[error("Authorization proof is invalid")]
    InvalidProof,
    #[error("Nonce has already been used (replay attack detected)")]
    NonceReused,
}

/// A 32-byte authorization token shared between vault operator and one agent.
///
/// Tokens are generated once during agent enrollment and are stored inside the
/// vault's sealed storage.  **They must never be transmitted to the agent in
/// plaintext after initial enrollment.**
#[derive(Clone)]
pub struct AuthToken(pub(crate) [u8; 32]);

impl AuthToken {
    /// Generate a fresh cryptographically random authorization token.
    pub fn generate() -> Self {
        let mut raw = [0u8; 32];
        getrandom::getrandom(&mut raw).expect("OS CSPRNG unavailable");
        Self(raw)
    }

    /// Create an `AuthToken` from known bytes (for testing / manual enrollment).
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return the raw token bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Drop for AuthToken {
    fn drop(&mut self) {
        for byte in self.0.iter_mut() {
            unsafe { std::ptr::write_volatile(byte as *mut u8, 0) };
        }
    }
}

/// A zero-knowledge-style authorization proof produced by the agent.
///
/// The agent computes:
///
/// ```text
/// commitment = HMAC-SHA256(key=auth_token, msg="zkcommit:v1:" ‖ nonce ‖ agent_id ‖ tx_digest)
/// ```
///
/// and sends `(nonce, commitment)` to the vault.  The vault reproduces the
/// computation with its copy of the token and accepts only if the commitment
/// matches.  The raw token is never transmitted.
#[derive(Debug, Clone)]
pub struct AgentAuthProof {
    /// Unique 32-byte value, generated fresh for every request.
    pub nonce: [u8; 32],
    /// HMAC-SHA256 commitment over (nonce, agent_id, tx_digest).
    pub commitment: [u8; 32],
}

impl AgentAuthProof {
    /// Build a proof given the agent's `auth_token`, a fresh `nonce`, the
    /// agent's DID string, and the SHA-256 digest of the transaction bytes.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::CommitmentBuild`] if the HMAC initialisation fails.
    pub fn build(
        auth_token: &AuthToken,
        nonce: [u8; 32],
        agent_id: &str,
        tx_digest: &[u8; 32],
    ) -> Result<Self, AuthError> {
        let commitment =
            ZkCommitmentScheme::commit(auth_token.as_bytes(), &nonce, agent_id, tx_digest)
                .map_err(|e| AuthError::CommitmentBuild(e.to_string()))?;
        Ok(Self { nonce, commitment })
    }

    /// Verify the proof against the vault's copy of the auth token.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidProof`] if the commitment does not match.
    pub fn verify(
        &self,
        auth_token: &AuthToken,
        agent_id: &str,
        tx_digest: &[u8; 32],
    ) -> Result<(), AuthError> {
        ZkCommitmentScheme::verify(
            auth_token.as_bytes(),
            &self.nonce,
            agent_id,
            tx_digest,
            &self.commitment,
        )
        .map_err(|_| AuthError::InvalidProof)
    }
}

/// Derive the SHA-256 digest of arbitrary bytes (used to hash transaction bytes
/// before including them in an auth proof).
pub fn tx_digest(tx_bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(tx_bytes);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_token() -> AuthToken {
        AuthToken::from_bytes([0x42u8; 32])
    }

    fn fresh_nonce() -> [u8; 32] {
        let mut n = [0u8; 32];
        getrandom::getrandom(&mut n).unwrap();
        n
    }

    #[test]
    fn proof_verifies_with_correct_token() {
        let token = sample_token();
        let nonce = fresh_nonce();
        let tx = b"fake-solana-transaction-bytes";
        let digest = tx_digest(tx);
        let proof = AgentAuthProof::build(&token, nonce, "did:ahin:agent:alice", &digest)
            .expect("build should succeed");
        proof
            .verify(&token, "did:ahin:agent:alice", &digest)
            .expect("verify should pass");
    }

    #[test]
    fn proof_fails_with_wrong_token() {
        let token = sample_token();
        let wrong_token = AuthToken::from_bytes([0x99u8; 32]);
        let nonce = fresh_nonce();
        let tx = b"fake-solana-transaction-bytes";
        let digest = tx_digest(tx);
        let proof = AgentAuthProof::build(&token, nonce, "did:ahin:agent:alice", &digest).unwrap();
        let result = proof.verify(&wrong_token, "did:ahin:agent:alice", &digest);
        assert!(matches!(result, Err(AuthError::InvalidProof)));
    }

    #[test]
    fn proof_fails_with_wrong_agent_id() {
        let token = sample_token();
        let nonce = fresh_nonce();
        let tx = b"fake-solana-transaction-bytes";
        let digest = tx_digest(tx);
        let proof = AgentAuthProof::build(&token, nonce, "did:ahin:agent:alice", &digest).unwrap();
        let result = proof.verify(&token, "did:ahin:agent:eve", &digest);
        assert!(matches!(result, Err(AuthError::InvalidProof)));
    }

    #[test]
    fn auth_token_generate_produces_32_bytes() {
        let token = AuthToken::generate();
        assert_eq!(token.as_bytes().len(), 32);
    }
}
