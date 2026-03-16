//! # TEE Foundation
//!
//! Hardware-rooted Trusted Execution Environment (TEE) primitives for the
//! Life++ full stack.
//!
//! This crate provides the lowest-level trust anchor on which every other
//! Life++ subsystem is built.  The abstractions here deliberately mirror the
//! hardware concepts found in real-world TEEs:
//!
//! | Concept               | Real hardware                          | This crate                   |
//! |-----------------------|----------------------------------------|------------------------------|
//! | Measurement           | AMD SEV-SNP `MEASUREMENT`, TPM PCR     | [`MeasurementChain`]         |
//! | Remote attestation    | AMD attestation report, SGX quote      | [`TeeReport`]                |
//! | Sealed storage        | AMD wrap-key, SGX seal                 | [`SealedBlob`]               |
//! | Enclave context       | PSP firmware, TrustZone Secure World   | [`TeeContext`]               |
//!
//! # Trust Model
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  Life++ Full Stack                                      │
//! │  ┌───────────────┐  ┌──────────────────────────────┐   │
//! │  │ Edge Runtime  │  │  CIS Genesis Provisioner     │   │
//! │  │ (Rust)        │  │  (Rust)                      │   │
//! │  └───────┬───────┘  └──────────────┬───────────────┘   │
//! │          │                         │                    │
//! │  ┌───────▼─────────────────────────▼───────────────┐   │
//! │  │          tee-foundation                         │   │
//! │  │  MeasurementChain · TeeReport · SealedBlob      │   │
//! │  └───────────────────────┬─────────────────────────┘   │
//! └──────────────────────────┼──────────────────────────── ┘
//!                            │
//!              ┌─────────────▼─────────────┐
//!              │   Hardware Security Root   │
//!              │  AMD SEV-SNP / TrustZone   │
//!              │  SRAM PUF / Apple SE       │
//!              └───────────────────────────┘
//! ```

pub mod attestation;
pub mod measurement;
pub mod sealed_storage;

pub use attestation::{TeeReport, TeeReportError, TeeVendor};
pub use measurement::{MeasurementChain, MeasurementError};
pub use sealed_storage::{SealedBlob, SealError};

use sha2::{Digest, Sha256};

/// A live TEE context.
///
/// `TeeContext` owns a [`MeasurementChain`] that records every software
/// component loaded into the enclave.  It can produce a [`TeeReport`] at any
/// point, and it can seal sensitive data blobs so they are cryptographically
/// bound to the current measurement state.
///
/// # Example
///
/// ```rust
/// use tee_foundation::TeeContext;
///
/// let mut ctx = TeeContext::new(tee_foundation::TeeVendor::AmdSevSnp);
/// ctx.extend_measurement(b"bootloader:v1.0");
/// ctx.extend_measurement(b"kernel:v5.15");
/// ctx.extend_measurement(b"life-plus-edge-runtime:v0.1.0");
///
/// let report = ctx.generate_report(b"user-data-nonce").unwrap();
/// assert!(report.verify().is_ok());
/// ```
pub struct TeeContext {
    vendor: TeeVendor,
    chain: MeasurementChain,
    /// Unique per-boot platform key derived from SRAM PUF entropy.
    platform_key: [u8; 32],
}

impl TeeContext {
    /// Create a new TEE context for the given hardware vendor.
    ///
    /// The platform key is derived freshly from OS-level entropy on every boot
    /// (in real hardware this comes from the silicon SRAM PUF).
    pub fn new(vendor: TeeVendor) -> Self {
        let platform_key = Self::derive_platform_key();
        Self {
            vendor,
            chain: MeasurementChain::new(),
            platform_key,
        }
    }

    /// Extend the software measurement with a new component digest.
    ///
    /// Call this once for every binary or configuration blob loaded into the
    /// enclave, in boot order.  The order is significant: swapping two
    /// components produces a completely different final measurement.
    pub fn extend_measurement(&mut self, component: &[u8]) {
        self.chain.extend(component);
    }

    /// Generate a remote-attestation report.
    ///
    /// `user_data` is caller-controlled data (e.g. a challenge nonce from the
    /// verifier) that is mixed into the report so it cannot be replayed.
    ///
    /// # Errors
    ///
    /// Returns [`TeeReportError`] if the measurement chain is empty (the
    /// enclave has not loaded any components yet).
    pub fn generate_report(&self, user_data: &[u8]) -> Result<TeeReport, TeeReportError> {
        TeeReport::generate(self.vendor, self.chain.digest(), user_data, &self.platform_key)
    }

    /// Seal `plaintext` to the **current** measurement state.
    ///
    /// The sealed blob can only be unsealed on a machine whose measurement
    /// chain produces the same digest as the one recorded at seal time.
    pub fn seal(&self, plaintext: &[u8]) -> SealedBlob {
        SealedBlob::seal(plaintext, &self.chain.digest(), &self.platform_key)
    }

    /// Attempt to unseal a [`SealedBlob`].
    ///
    /// Succeeds only when the current measurement digest matches the one that
    /// was present when the blob was sealed.
    ///
    /// # Errors
    ///
    /// Returns [`SealError::MeasurementMismatch`] if the measurement has
    /// changed since the blob was sealed (i.e. the software running is not the
    /// exact same stack that sealed the data).
    pub fn unseal(&self, blob: &SealedBlob) -> Result<Vec<u8>, SealError> {
        blob.unseal(&self.chain.digest(), &self.platform_key)
    }

    /// Return the current measurement digest as a hex string.
    pub fn measurement_hex(&self) -> String {
        hex::encode(self.chain.digest())
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Derive a per-boot platform key from OS entropy (SRAM PUF seed in
    /// production hardware).
    fn derive_platform_key() -> [u8; 32] {
        let mut raw = [0u8; 32];
        // On bare-metal this would read from the AMD PSP / Apple ANE key store.
        // In software we use the OS CSPRNG which provides equivalent entropy.
        getrandom::getrandom(&mut raw).expect("OS CSPRNG unavailable");
        let mut h = Sha256::new();
        h.update(b"tee:platform-key:v1:");
        h.update(raw);
        h.finalize().into()
    }
}
