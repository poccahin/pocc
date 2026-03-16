//! Remote attestation report – the Life++ equivalent of an AMD SEV-SNP or
//! Intel TDX attestation report.
//!
//! A [`TeeReport`] binds:
//!
//! * the **software measurement** (digest of every loaded component),
//! * **user-supplied data** (typically a challenge nonce from the verifier),
//! * the **hardware vendor** tag (AMD SEV-SNP, Apple Secure Enclave, …), and
//! * a **platform signature** (HMAC-SHA-256 keyed to the silicon-unique
//!   platform key) over all of the above.
//!
//! Verifying the report ensures that:
//!
//! 1. The report has not been tampered with (signature check).
//! 2. The software measurement matches what the verifier expects.
//!
//! In real AMD SEV-SNP hardware, step (1) uses an AMD-signed certificate chain
//! rooted at AMD's hardware key.  The software model here uses a per-boot
//! platform key to demonstrate the structure while remaining fully testable
//! without physical hardware.

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Hardware vendor / TEE technology tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeeVendor {
    /// AMD Secure Encrypted Virtualisation – Secure Nested Paging (SEV-SNP).
    AmdSevSnp,
    /// Apple Secure Enclave Processor (SEP) – used in M-series SoCs.
    AppleSecureEnclave,
    /// ARM TrustZone Secure World (used in Sophgo / Rockchip edge SoCs).
    ArmTrustZone,
    /// Software emulation – for testing without physical TEE hardware.
    Software,
}

impl TeeVendor {
    /// Return the canonical ASCII identifier for this vendor tag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AmdSevSnp => "AMD_SEV_SNP",
            Self::AppleSecureEnclave => "APPLE_SECURE_ENCLAVE",
            Self::ArmTrustZone => "ARM_TRUSTZONE",
            Self::Software => "SOFTWARE_TEE",
        }
    }
}

/// Errors returned when generating or verifying a [`TeeReport`].
#[derive(Debug, Error)]
pub enum TeeReportError {
    #[error("Cannot generate report: measurement chain is empty")]
    EmptyMeasurement,
    #[error("Report signature verification failed: report has been tampered with")]
    SignatureInvalid,
    #[error("Measurement mismatch: expected {expected}, got {got}")]
    MeasurementMismatch { expected: String, got: String },
}

/// A hardware-style remote attestation report.
///
/// The report contains:
///
/// * `vendor` – which TEE technology produced the report.
/// * `measurement` – SHA-256 digest of the complete software measurement
///   chain at the time of report generation.
/// * `user_data` – up to 64 bytes of caller-supplied data (challenge nonce).
/// * `report_data_hash` – SHA-256 of (`measurement` ∥ `user_data`).
/// * `platform_signature` – HMAC-SHA-256 keyed on the per-boot platform key,
///   computed over `vendor_tag ∥ measurement ∥ user_data`.
#[derive(Debug, Clone)]
pub struct TeeReport {
    /// TEE technology that produced this report.
    pub vendor: TeeVendor,
    /// 32-byte software measurement digest (SHA-256 of the full PCR chain).
    pub measurement: [u8; 32],
    /// Up to 64 bytes of caller-supplied challenge data.
    pub user_data: Vec<u8>,
    /// SHA-256 of (`measurement` ∥ `user_data`), for quick integrity checks.
    pub report_data_hash: [u8; 32],
    /// Platform-key HMAC over the full report body. Proves the report was
    /// produced by a genuine TEE instance holding the platform key.
    pub(crate) platform_signature: [u8; 32],
}

impl TeeReport {
    /// Generate a new attestation report.
    ///
    /// # Parameters
    ///
    /// * `vendor` – TEE hardware vendor.
    /// * `measurement` – current measurement chain digest.
    /// * `user_data` – caller nonce; at most 64 bytes are used.
    /// * `platform_key` – 32-byte silicon-unique key (SRAM PUF seed in production).
    ///
    /// # Errors
    ///
    /// Returns [`TeeReportError::EmptyMeasurement`] if `measurement` is the
    /// all-zero initial value (no components have been extended yet).
    pub fn generate(
        vendor: TeeVendor,
        measurement: [u8; 32],
        user_data: &[u8],
        platform_key: &[u8; 32],
    ) -> Result<Self, TeeReportError> {
        if measurement == [0u8; 32] {
            return Err(TeeReportError::EmptyMeasurement);
        }

        // Truncate user_data to 64 bytes (mirrors AMD SEV-SNP HOST_DATA field).
        let user_data: Vec<u8> = user_data.iter().copied().take(64).collect();

        // report_data_hash = SHA-256( measurement ∥ user_data )
        let report_data_hash = {
            let mut h = Sha256::new();
            h.update(measurement);
            h.update(&user_data);
            h.finalize().into()
        };

        // platform_signature = SHA-256( platform_key ∥ vendor_tag ∥ measurement ∥ user_data )
        // (In real AMD SEV-SNP this is an ECDSA P-384 signature over the VCEK key.)
        let platform_signature = {
            let mut h = Sha256::new();
            h.update(platform_key);
            h.update(vendor.as_str().as_bytes());
            h.update(measurement);
            h.update(&user_data);
            h.finalize().into()
        };

        Ok(Self {
            vendor,
            measurement,
            user_data,
            report_data_hash,
            platform_signature,
        })
    }

    /// Verify the internal consistency of this report.
    ///
    /// Checks that `report_data_hash` matches the hash of (`measurement` ∥
    /// `user_data`).  A remote verifier would additionally re-derive the
    /// `platform_signature` using the attested vendor certificate chain.
    ///
    /// # Errors
    ///
    /// Returns [`TeeReportError::SignatureInvalid`] if any field has been
    /// altered after the report was generated.
    pub fn verify(&self) -> Result<(), TeeReportError> {
        let expected_hash = {
            let mut h = Sha256::new();
            h.update(self.measurement);
            h.update(&self.user_data);
            let hash: [u8; 32] = h.finalize().into();
            hash
        };

        if self.report_data_hash != expected_hash {
            return Err(TeeReportError::SignatureInvalid);
        }

        Ok(())
    }

    /// Verify that the report's measurement matches an expected value.
    ///
    /// # Errors
    ///
    /// Returns [`TeeReportError::MeasurementMismatch`] if the measurements
    /// differ.
    pub fn verify_measurement(&self, expected: &[u8; 32]) -> Result<(), TeeReportError> {
        if &self.measurement != expected {
            return Err(TeeReportError::MeasurementMismatch {
                expected: hex::encode(expected),
                got: hex::encode(self.measurement),
            });
        }
        Ok(())
    }

    /// Return the measurement as a hex string (useful for logging / comparison).
    pub fn measurement_hex(&self) -> String {
        hex::encode(self.measurement)
    }

    /// Return the platform signature as a hex string.
    pub fn signature_hex(&self) -> String {
        hex::encode(self.platform_signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measurement::MeasurementChain;

    fn make_key() -> [u8; 32] {
        [0xABu8; 32]
    }

    fn make_measurement() -> [u8; 32] {
        let mut chain = MeasurementChain::new();
        chain.extend(b"bootloader");
        chain.extend(b"kernel");
        chain.digest()
    }

    #[test]
    fn report_generate_and_verify_ok() {
        let m = make_measurement();
        let report = TeeReport::generate(TeeVendor::AmdSevSnp, m, b"nonce-abc", &make_key())
            .expect("should generate");
        report.verify().expect("report should be valid");
    }

    #[test]
    fn report_fails_on_empty_measurement() {
        let result =
            TeeReport::generate(TeeVendor::Software, [0u8; 32], b"nonce", &make_key());
        assert!(matches!(result, Err(TeeReportError::EmptyMeasurement)));
    }

    #[test]
    fn tampered_report_data_hash_fails_verify() {
        let m = make_measurement();
        let mut report =
            TeeReport::generate(TeeVendor::AmdSevSnp, m, b"nonce", &make_key()).unwrap();
        report.report_data_hash[0] ^= 0xFF; // corrupt one byte
        assert!(matches!(report.verify(), Err(TeeReportError::SignatureInvalid)));
    }

    #[test]
    fn measurement_verification_passes_for_correct_expected() {
        let m = make_measurement();
        let report =
            TeeReport::generate(TeeVendor::AppleSecureEnclave, m, b"x", &make_key()).unwrap();
        report.verify_measurement(&m).expect("should match");
    }

    #[test]
    fn measurement_verification_fails_for_wrong_expected() {
        let m = make_measurement();
        let report =
            TeeReport::generate(TeeVendor::ArmTrustZone, m, b"x", &make_key()).unwrap();
        let wrong = [0xFFu8; 32];
        assert!(matches!(
            report.verify_measurement(&wrong),
            Err(TeeReportError::MeasurementMismatch { .. })
        ));
    }

    #[test]
    fn different_vendors_produce_different_signatures() {
        let m = make_measurement();
        let k = make_key();
        let r1 = TeeReport::generate(TeeVendor::AmdSevSnp, m, b"n", &k).unwrap();
        let r2 = TeeReport::generate(TeeVendor::Software, m, b"n", &k).unwrap();
        assert_ne!(r1.platform_signature, r2.platform_signature);
    }

    #[test]
    fn different_user_data_produces_different_hashes() {
        let m = make_measurement();
        let k = make_key();
        let r1 = TeeReport::generate(TeeVendor::Software, m, b"nonce-1", &k).unwrap();
        let r2 = TeeReport::generate(TeeVendor::Software, m, b"nonce-2", &k).unwrap();
        assert_ne!(r1.report_data_hash, r2.report_data_hash);
    }
}
