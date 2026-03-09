//! SIWA (Sign-In With Agent) verification helpers.
//! This module models ERC-8004/8128 style challenge-response authentication
//! and can be swapped with a production verifier once SDKs are integrated.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct SiwaClaims {
    pub agent_id: String,
    pub issued_at_unix: u64,
    pub nonce: String,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct SiwaVerifier {
    max_clock_skew_secs: u64,
}

impl SiwaVerifier {
    pub fn new(max_clock_skew_secs: u64) -> Self {
        Self {
            max_clock_skew_secs,
        }
    }

    /// For now we model SIWA signature as:
    /// hex(sha256("siwa:<agent_id>:<nonce>:<issued_at>"))
    /// In production this path should verify secp256k1/erc-1271 signatures.
    pub fn verify(&self, claims: &SiwaClaims, now_unix: u64) -> Result<(), &'static str> {
        if claims.agent_id.trim().is_empty() {
            return Err("SIWA_REJECTED: empty agent id");
        }

        if now_unix.saturating_sub(claims.issued_at_unix) > self.max_clock_skew_secs {
            return Err("SIWA_REJECTED: stale SIWA challenge");
        }

        let expected = expected_signature(&claims.agent_id, &claims.nonce, claims.issued_at_unix);
        if expected != claims.signature {
            return Err("SIWA_REJECTED: signature mismatch");
        }

        Ok(())
    }
}

pub fn expected_signature(agent_id: &str, nonce: &str, issued_at_unix: u64) -> String {
    let msg = format!("siwa:{agent_id}:{nonce}:{issued_at_unix}");
    let digest = Sha256::digest(msg.as_bytes());
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_siwa() {
        let verifier = SiwaVerifier::new(300);
        let issued = 1000;
        let claims = SiwaClaims {
            agent_id: "cai-1".to_string(),
            issued_at_unix: issued,
            nonce: "abc".to_string(),
            signature: expected_signature("cai-1", "abc", issued),
        };

        assert!(verifier.verify(&claims, 1200).is_ok());
    }

    #[test]
    fn rejects_bad_signature() {
        let verifier = SiwaVerifier::new(300);
        let claims = SiwaClaims {
            agent_id: "cai-1".to_string(),
            issued_at_unix: 1000,
            nonce: "abc".to_string(),
            signature: "bad".to_string(),
        };

        assert!(verifier.verify(&claims, 1200).is_err());
    }
}
