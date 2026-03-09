//! Edge-side TEE envelope mock.
//! Models key-isolation and attestation checks before privileged operations.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct TeeEnvelope {
    expected_measurement: String,
}

#[derive(Debug, Clone)]
pub struct TeeAttestation {
    pub measurement: String,
    pub quote_hash: String,
}

impl TeeEnvelope {
    pub fn new(expected_measurement: impl Into<String>) -> Self {
        Self {
            expected_measurement: expected_measurement.into(),
        }
    }

    pub fn verify_attestation(&self, attestation: &TeeAttestation) -> Result<(), &'static str> {
        if attestation.measurement != self.expected_measurement {
            return Err("TEE_REJECTED: enclave measurement mismatch");
        }

        if attestation.quote_hash.len() < 16 {
            return Err("TEE_REJECTED: invalid quote hash");
        }

        Ok(())
    }

    /// Simulated enclave signing routine; caller only sees signature, not key.
    pub fn sign_inside_enclave(&self, payload: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(self.expected_measurement.as_bytes());
        h.update(payload);
        hex::encode(h.finalize())
    }
}
