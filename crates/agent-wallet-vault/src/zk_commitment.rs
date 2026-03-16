//! Zero-knowledge commitment scheme for agent authorization.
//!
//! An agent proves knowledge of its `auth_token` via an HMAC commitment:
//!
//! ```text
//! commitment = HMAC-SHA256(key=auth_token, msg=nonce ‖ agent_id ‖ tx_digest)
//! ```
//!
//! The vault holds the same `auth_token` and can verify the commitment without
//! the agent ever sending the raw token.  This is a "proof of knowledge of a
//! pre-image" implemented via a keyed hash, equivalent to a Sigma protocol
//! without the circuit overhead, and is quantum-safe (HMAC-SHA256 retains
//! 128-bit security against Grover's algorithm).

use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

/// Errors that can arise during commitment generation or verification.
#[derive(Debug, Error)]
pub enum CommitmentError {
    #[error("HMAC key initialisation failed")]
    KeyInit,
    #[error("Commitment verification failed: expected {expected}, got {got}")]
    VerificationFailed { expected: String, got: String },
}

/// Stateless helper that issues and verifies HMAC-SHA256 commitments.
///
/// Both the prover (agent) and the verifier (vault) use this struct with the
/// **same** `auth_token` seed to ensure the vault can recompute the expected
/// commitment without the token ever crossing the trust boundary.
pub struct ZkCommitmentScheme;

impl ZkCommitmentScheme {
    /// Compute a 32-byte HMAC-SHA256 commitment.
    ///
    /// `auth_token` – the shared secret known to agent and vault.  
    /// `nonce`      – unique per-request bytes (prevents replay).  
    /// `agent_id`   – ASCII agent DID string.  
    /// `tx_digest`  – SHA-256 digest of the transaction bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CommitmentError::KeyInit`] if the HMAC key cannot be
    /// initialised (should never happen with a non-empty token).
    pub fn commit(
        auth_token: &[u8],
        nonce: &[u8; 32],
        agent_id: &str,
        tx_digest: &[u8; 32],
    ) -> Result<[u8; 32], CommitmentError> {
        let mut mac =
            HmacSha256::new_from_slice(auth_token).map_err(|_| CommitmentError::KeyInit)?;
        mac.update(b"zkcommit:v1:");
        mac.update(nonce);
        mac.update(agent_id.as_bytes());
        mac.update(tx_digest);
        let result = mac.finalize().into_bytes();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        Ok(out)
    }

    /// Verify a commitment produced by [`ZkCommitmentScheme::commit`].
    ///
    /// Uses constant-time comparison to prevent timing side-channels.
    ///
    /// # Errors
    ///
    /// Returns [`CommitmentError::VerificationFailed`] if the commitment does
    /// not match the expected value.
    pub fn verify(
        auth_token: &[u8],
        nonce: &[u8; 32],
        agent_id: &str,
        tx_digest: &[u8; 32],
        commitment: &[u8; 32],
    ) -> Result<(), CommitmentError> {
        let expected = Self::commit(auth_token, nonce, agent_id, tx_digest)?;
        if !constant_time_eq(&expected, commitment) {
            return Err(CommitmentError::VerificationFailed {
                expected: hex::encode(expected),
                got: hex::encode(commitment),
            });
        }
        Ok(())
    }
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_token() -> Vec<u8> {
        b"super-secret-auth-token-32bytes!".to_vec()
    }

    fn nonce() -> [u8; 32] {
        [0x01u8; 32]
    }

    fn digest() -> [u8; 32] {
        [0xABu8; 32]
    }

    #[test]
    fn commitment_verifies_correctly() {
        let token = sample_token();
        let n = nonce();
        let d = digest();
        let commitment = ZkCommitmentScheme::commit(&token, &n, "did:ahin:agent:test", &d)
            .expect("commit should succeed");
        ZkCommitmentScheme::verify(&token, &n, "did:ahin:agent:test", &d, &commitment)
            .expect("verify should pass");
    }

    #[test]
    fn wrong_token_fails_verification() {
        let token = sample_token();
        let bad_token = b"wrong-token-entirely-different!".to_vec();
        let n = nonce();
        let d = digest();
        let commitment = ZkCommitmentScheme::commit(&token, &n, "did:ahin:agent:test", &d)
            .expect("commit should succeed");
        let result = ZkCommitmentScheme::verify(&bad_token, &n, "did:ahin:agent:test", &d, &commitment);
        assert!(result.is_err());
    }

    #[test]
    fn wrong_nonce_fails_verification() {
        let token = sample_token();
        let n = nonce();
        let bad_nonce = [0x02u8; 32];
        let d = digest();
        let commitment = ZkCommitmentScheme::commit(&token, &n, "did:ahin:agent:test", &d)
            .expect("commit should succeed");
        let result = ZkCommitmentScheme::verify(&token, &bad_nonce, "did:ahin:agent:test", &d, &commitment);
        assert!(result.is_err());
    }

    #[test]
    fn wrong_agent_id_fails_verification() {
        let token = sample_token();
        let n = nonce();
        let d = digest();
        let commitment = ZkCommitmentScheme::commit(&token, &n, "did:ahin:agent:alice", &d)
            .expect("commit should succeed");
        let result = ZkCommitmentScheme::verify(&token, &n, "did:ahin:agent:bob", &d, &commitment);
        assert!(result.is_err());
    }

    #[test]
    fn commitment_is_deterministic() {
        let token = sample_token();
        let n = nonce();
        let d = digest();
        let c1 = ZkCommitmentScheme::commit(&token, &n, "did:ahin:agent:test", &d).unwrap();
        let c2 = ZkCommitmentScheme::commit(&token, &n, "did:ahin:agent:test", &d).unwrap();
        assert_eq!(c1, c2);
    }
}
