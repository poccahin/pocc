//! TEE-sealed storage – data cryptographically bound to the measurement state.
//!
//! A [`SealedBlob`] wraps plaintext so it can **only** be recovered when the
//! software running inside the enclave has the exact same measurement chain
//! digest as it had when the data was sealed.
//!
//! This mirrors the AMD SEV-SNP wrap-key / Intel SGX `sgx_seal_data` API.  In
//! real hardware the sealing key is derived from the hardware platform key and
//! the MEASUREMENT register; any change to the loaded software stack produces a
//! different sealing key and decryption fails.
//!
//! # Security properties
//!
//! * **Measurement binding** – if the software changes (new firmware version,
//!   different configuration, additional component loaded), the measurement
//!   digest changes and the blob cannot be unsealed.
//! * **Platform binding** – the platform key is generated freshly from SRAM PUF
//!   entropy on every cold boot; a different physical device cannot unseal a
//!   blob produced on another device.
//! * **Integrity protection** – a 32-byte authenticator tag detects any
//!   tampering with the ciphertext.

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors returned when sealing or unsealing data.
#[derive(Debug, Error)]
pub enum SealError {
    #[error(
        "Unseal failed: measurement mismatch – the enclave software stack has \
         changed since the blob was sealed"
    )]
    MeasurementMismatch,
    #[error("Unseal failed: authentication tag invalid – the blob has been tampered with")]
    AuthTagInvalid,
    #[error("Unseal failed: ciphertext is shorter than the minimum valid length")]
    CiphertextTooShort,
}

/// A measurement-bound sealed data blob.
///
/// The internal layout is:
///
/// ```text
/// ┌──────────────────────────────────────────┐
/// │  measurement_tag  [32 bytes]             │  SHA-256(platform_key ∥ measurement)
/// │  auth_tag         [32 bytes]             │  SHA-256(seal_key ∥ ciphertext)
/// │  ciphertext       [variable]             │  plaintext XOR keystream
/// └──────────────────────────────────────────┘
/// ```
///
/// The `measurement_tag` acts as a fast-fail check: if the current measurement
/// does not reproduce the same tag, decryption is skipped entirely.
#[derive(Debug, Clone)]
pub struct SealedBlob {
    /// SHA-256(platform_key ∥ measurement) – quick measurement check.
    measurement_tag: [u8; 32],
    /// Authentication tag over the ciphertext.
    auth_tag: [u8; 32],
    /// Encrypted payload.
    ciphertext: Vec<u8>,
}

impl SealedBlob {
    /// Seal `plaintext` using `measurement` and `platform_key`.
    ///
    /// The function derives a sealing key as:
    ///
    /// ```text
    /// seal_key = SHA-256( "tee:seal:v1:" ∥ platform_key ∥ measurement )
    /// ```
    ///
    /// Encryption is XOR with a SHA-256-derived keystream (stream cipher
    /// approximation suitable for the software TEE model; production hardware
    /// would use AES-256-GCM via the PSP crypto engine).
    pub fn seal(plaintext: &[u8], measurement: &[u8; 32], platform_key: &[u8; 32]) -> Self {
        let seal_key = Self::derive_seal_key(measurement, platform_key);
        let measurement_tag = Self::derive_measurement_tag(measurement, platform_key);

        let ciphertext = Self::xor_stream(plaintext, &seal_key);

        let auth_tag = {
            let mut h = Sha256::new();
            h.update(seal_key);
            h.update(&ciphertext);
            h.finalize().into()
        };

        Self {
            measurement_tag,
            auth_tag,
            ciphertext,
        }
    }

    /// Attempt to unseal the blob.
    ///
    /// # Errors
    ///
    /// * [`SealError::MeasurementMismatch`] – the current measurement digest
    ///   does not match the one used at seal time.
    /// * [`SealError::AuthTagInvalid`] – the blob has been tampered with.
    /// * [`SealError::CiphertextTooShort`] – internal invariant violation.
    pub fn unseal(
        &self,
        measurement: &[u8; 32],
        platform_key: &[u8; 32],
    ) -> Result<Vec<u8>, SealError> {
        // Fast-fail: check the measurement tag before any crypto work.
        let expected_tag = Self::derive_measurement_tag(measurement, platform_key);
        if !constant_time_eq(&self.measurement_tag, &expected_tag) {
            return Err(SealError::MeasurementMismatch);
        }

        let seal_key = Self::derive_seal_key(measurement, platform_key);

        // Verify the authentication tag.
        let expected_auth = {
            let mut h = Sha256::new();
            h.update(seal_key);
            h.update(&self.ciphertext);
            let hash: [u8; 32] = h.finalize().into();
            hash
        };
        if !constant_time_eq(&self.auth_tag, &expected_auth) {
            return Err(SealError::AuthTagInvalid);
        }

        Ok(Self::xor_stream(&self.ciphertext, &seal_key))
    }

    /// Serialize the sealed blob into an opaque byte string for storage.
    ///
    /// Layout: `measurement_tag (32) ‖ auth_tag (32) ‖ ciphertext (variable)`.
    ///
    /// Use [`SealedBlob::from_bytes`] to reconstruct the blob for unsealing.
    pub fn into_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.ciphertext.len());
        out.extend_from_slice(&self.measurement_tag);
        out.extend_from_slice(&self.auth_tag);
        out.extend_from_slice(&self.ciphertext);
        out
    }

    /// Reconstruct a [`SealedBlob`] from its serialised byte representation.
    ///
    /// # Errors
    ///
    /// Returns [`SealError::CiphertextTooShort`] if `bytes` is shorter than
    /// the 64-byte header (32-byte measurement tag + 32-byte auth tag).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SealError> {
        if bytes.len() < 64 {
            return Err(SealError::CiphertextTooShort);
        }
        let mut measurement_tag = [0u8; 32];
        let mut auth_tag = [0u8; 32];
        measurement_tag.copy_from_slice(&bytes[..32]);
        auth_tag.copy_from_slice(&bytes[32..64]);
        let ciphertext = bytes[64..].to_vec();
        Ok(Self {
            measurement_tag,
            auth_tag,
            ciphertext,
        })
    }


    fn derive_seal_key(measurement: &[u8; 32], platform_key: &[u8; 32]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"tee:seal:v1:");
        h.update(platform_key);
        h.update(measurement);
        h.finalize().into()
    }

    fn derive_measurement_tag(measurement: &[u8; 32], platform_key: &[u8; 32]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"tee:mtag:v1:");
        h.update(platform_key);
        h.update(measurement);
        h.finalize().into()
    }

    /// XOR-stream cipher: output[i] = input[i] XOR keystream[i % 32].
    ///
    /// In production this is replaced by AES-256-GCM on the PSP hardware
    /// crypto engine, which provides both confidentiality and authentication.
    fn xor_stream(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % 32])
            .collect()
    }
}

/// Constant-time byte-slice equality to prevent timing side-channels.
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
    use crate::measurement::MeasurementChain;

    fn measurement() -> [u8; 32] {
        let mut c = MeasurementChain::new();
        c.extend(b"firmware:v1");
        c.extend(b"runtime:v0.1");
        c.digest()
    }

    fn platform_key() -> [u8; 32] {
        [0xCAu8; 32]
    }

    #[test]
    fn seal_then_unseal_recovers_plaintext() {
        let plain = b"super-secret-key-material";
        let m = measurement();
        let k = platform_key();
        let blob = SealedBlob::seal(plain, &m, &k);
        let recovered = blob.unseal(&m, &k).expect("should unseal");
        assert_eq!(recovered, plain);
    }

    #[test]
    fn wrong_measurement_fails_unseal() {
        let plain = b"data";
        let m = measurement();
        let k = platform_key();
        let blob = SealedBlob::seal(plain, &m, &k);

        let mut bad_m = m;
        bad_m[0] ^= 0x01;
        assert!(matches!(blob.unseal(&bad_m, &k), Err(SealError::MeasurementMismatch)));
    }

    #[test]
    fn wrong_platform_key_fails_unseal() {
        let plain = b"data";
        let m = measurement();
        let k = platform_key();
        let blob = SealedBlob::seal(plain, &m, &k);

        let mut bad_k = k;
        bad_k[0] ^= 0xFF;
        assert!(matches!(blob.unseal(&m, &bad_k), Err(SealError::MeasurementMismatch)));
    }

    #[test]
    fn tampered_ciphertext_fails_auth_check() {
        let plain = b"data";
        let m = measurement();
        let k = platform_key();
        let mut blob = SealedBlob::seal(plain, &m, &k);
        blob.ciphertext[0] ^= 0x01; // corrupt one byte
        assert!(matches!(blob.unseal(&m, &k), Err(SealError::AuthTagInvalid)));
    }

    #[test]
    fn empty_plaintext_round_trips() {
        let m = measurement();
        let k = platform_key();
        let blob = SealedBlob::seal(&[], &m, &k);
        let recovered = blob.unseal(&m, &k).expect("should unseal empty");
        assert!(recovered.is_empty());
    }

    #[test]
    fn sealed_blobs_from_different_measurements_cannot_cross_unseal() {
        let mut c1 = MeasurementChain::new();
        c1.extend(b"v1");
        let mut c2 = MeasurementChain::new();
        c2.extend(b"v2");
        let k = platform_key();

        let b1 = SealedBlob::seal(b"secret", &c1.digest(), &k);
        assert!(b1.unseal(&c2.digest(), &k).is_err());
    }
}
