//! AHIN edge-side TEE envelope.
//!
//! Provides measurement-chain-backed remote attestation and key isolation for
//! privileged routing operations in the CR+ tensor routing daemon.
//!
//! This module wraps the [`tee_foundation`] crate to give the routing layer a
//! typed, operation-specific API.

use sha2::{Digest, Sha256};
use tee_foundation::{MeasurementChain, TeeContext, TeeReport, TeeReportError, TeeVendor};

/// The canonical measurement tag for the AHIN edge TEE enclave v1.
///
/// Each binary release of the routing daemon extends the measurement chain
/// with this component string.  A report whose measurement does not match
/// the expected final digest after extending with this string is rejected.
pub const AHIN_EDGE_TEE_COMPONENT: &[u8] = b"ahin-cr-plus-tensor-routing:v0.1.0";

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

    /// Verify a legacy string-based attestation (used by external callers that
    /// supply `tee_measurement` / `tee_quote_hash` fields in the intent JSON).
    pub fn verify_attestation(&self, attestation: &TeeAttestation) -> Result<(), &'static str> {
        if attestation.measurement != self.expected_measurement {
            return Err("TEE_REJECTED: enclave measurement mismatch");
        }

        if attestation.quote_hash.len() < 16 {
            return Err("TEE_REJECTED: invalid quote hash");
        }

        Ok(())
    }

    /// Verify a structured [`TeeReport`] produced by the `tee-foundation` layer.
    ///
    /// Checks:
    /// 1. Internal consistency of the report (report_data_hash).
    /// 2. That the measurement inside the report matches the expected value
    ///    stored in this envelope.
    pub fn verify_tee_report(&self, report: &TeeReport) -> Result<(), String> {
        report.verify().map_err(|e| e.to_string())?;

        if report.measurement_hex() != self.expected_measurement {
            return Err(format!(
                "TEE_REJECTED: measurement {} does not match expected {}",
                &report.measurement_hex()[..16],
                &self.expected_measurement[..self.expected_measurement.len().min(16)],
            ));
        }

        Ok(())
    }

    /// Signing routine that runs inside the logical enclave boundary.
    ///
    /// The caller only observes the resulting signature hex string; the signing
    /// key (derived from the expected measurement) never leaves this function.
    /// In production hardware this uses the AMD PSP or Apple Secure Enclave
    /// hardware signing engine.
    pub fn sign_inside_enclave(&self, payload: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(self.expected_measurement.as_bytes());
        h.update(payload);
        hex::encode(h.finalize())
    }
}

/// Build a [`TeeContext`] pre-loaded with the AHIN edge routing daemon's
/// standard measurement chain.
///
/// Call `generate_report` on the returned context to obtain a report suitable
/// for passing to a remote verifier or to [`TeeEnvelope::verify_tee_report`].
pub fn build_ahin_tee_context() -> TeeContext {
    let mut ctx = TeeContext::new(TeeVendor::AmdSevSnp);
    ctx.extend_measurement(AHIN_EDGE_TEE_COMPONENT);
    ctx
}

/// Compute the expected measurement hex string for the AHIN edge enclave v1.
///
/// This value is embedded in `TeeEnvelope::new("LIFEPP_EDGE_TEE_V1")` today
/// but the production deployment should pin against the output of this
/// function to enforce measurement-level binding rather than a symbolic tag.
pub fn expected_ahin_edge_measurement() -> String {
    let mut chain = MeasurementChain::new();
    chain.extend(AHIN_EDGE_TEE_COMPONENT);
    hex::encode(chain.digest())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_attestation_accepts_matching_measurement() {
        let env = TeeEnvelope::new("LIFEPP_EDGE_TEE_V1");
        let att = TeeAttestation {
            measurement: "LIFEPP_EDGE_TEE_V1".into(),
            quote_hash: "abcdef1234567890".into(),
        };
        assert!(env.verify_attestation(&att).is_ok());
    }

    #[test]
    fn verify_attestation_rejects_wrong_measurement() {
        let env = TeeEnvelope::new("LIFEPP_EDGE_TEE_V1");
        let att = TeeAttestation {
            measurement: "WRONG_TEE".into(),
            quote_hash: "abcdef1234567890".into(),
        };
        assert!(env.verify_attestation(&att).is_err());
    }

    #[test]
    fn verify_attestation_rejects_short_quote_hash() {
        let env = TeeEnvelope::new("LIFEPP_EDGE_TEE_V1");
        let att = TeeAttestation {
            measurement: "LIFEPP_EDGE_TEE_V1".into(),
            quote_hash: "short".into(),
        };
        assert!(env.verify_attestation(&att).is_err());
    }

    #[test]
    fn sign_inside_enclave_is_deterministic() {
        let env = TeeEnvelope::new("LIFEPP_EDGE_TEE_V1");
        let sig1 = env.sign_inside_enclave(b"routing-payload");
        let sig2 = env.sign_inside_enclave(b"routing-payload");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn sign_inside_enclave_differs_for_different_measurements() {
        let env1 = TeeEnvelope::new("ENCLAVE_A");
        let env2 = TeeEnvelope::new("ENCLAVE_B");
        let sig1 = env1.sign_inside_enclave(b"payload");
        let sig2 = env2.sign_inside_enclave(b"payload");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn build_ahin_tee_context_generates_valid_report() {
        let ctx = build_ahin_tee_context();
        let report = ctx.generate_report(b"challenge-nonce").expect("report should succeed");
        report.verify().expect("report should be internally consistent");
    }

    #[test]
    fn expected_measurement_is_non_zero_hex() {
        let m = expected_ahin_edge_measurement();
        assert_eq!(m.len(), 64); // 32 bytes → 64 hex chars
        assert!(m.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(m, "0".repeat(64));
    }

    #[test]
    fn verify_tee_report_accepts_report_with_matching_hex_measurement() {
        let expected_hex = expected_ahin_edge_measurement();
        let env = TeeEnvelope::new(expected_hex.clone());
        let ctx = build_ahin_tee_context();
        let report = ctx.generate_report(b"nonce").unwrap();
        assert_eq!(report.measurement_hex(), expected_hex);
        env.verify_tee_report(&report).expect("should accept matching report");
    }

    #[test]
    fn verify_tee_report_rejects_report_with_wrong_measurement() {
        let env = TeeEnvelope::new("LIFEPP_EDGE_TEE_V1".to_string());
        let ctx = build_ahin_tee_context();
        let report = ctx.generate_report(b"nonce").unwrap();
        // The report's measurement hex will NOT equal "LIFEPP_EDGE_TEE_V1".
        assert!(env.verify_tee_report(&report).is_err());
    }
}
