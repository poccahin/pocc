//! Software measurement chain – the Life++ equivalent of TPM PCR-extend.
//!
//! A measurement chain records every software component loaded into the trusted
//! execution context.  Each [`extend`](MeasurementChain::extend) call
//! irrevocably folds a new component hash into the running digest:
//!
//! ```text
//! digest[n+1] = SHA-256( digest[n] || SHA-256(component) )
//! ```
//!
//! This mirrors the TPM PCR-extend operation and ensures that:
//!
//! * **Order matters** – loading component A then B yields a different digest
//!   than loading B then A.
//! * **Omission is detected** – skipping any component changes the final
//!   measurement.
//! * **The chain is append-only** – there is no way to remove a component once
//!   it has been extended.

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors that can be returned by measurement operations.
#[derive(Debug, Error)]
pub enum MeasurementError {
    #[error("Measurement chain is empty – no components have been extended yet")]
    EmptyChain,
}

/// A running software measurement chain (TPM PCR-extend equivalent).
///
/// The chain is initialised with an all-zero 32-byte digest, and each call to
/// [`extend`](Self::extend) advances the chain using the formula:
///
/// ```text
/// new_digest = SHA-256( current_digest || SHA-256(component) )
/// ```
#[derive(Clone, Debug)]
pub struct MeasurementChain {
    digest: [u8; 32],
    component_count: usize,
}

impl MeasurementChain {
    /// Create a new measurement chain in its initial all-zero state.
    pub fn new() -> Self {
        Self {
            digest: [0u8; 32],
            component_count: 0,
        }
    }

    /// Extend the chain with a new software component.
    ///
    /// `component` is typically the raw bytes of a firmware image, a Merkle
    /// root, or a deterministic serialisation of a configuration object.
    pub fn extend(&mut self, component: &[u8]) {
        // 1. Hash the component itself.
        let mut comp_hasher = Sha256::new();
        comp_hasher.update(component);
        let comp_hash: [u8; 32] = comp_hasher.finalize().into();

        // 2. Fold into the running digest: new = SHA-256(prev || comp_hash).
        let mut chain_hasher = Sha256::new();
        chain_hasher.update(self.digest);
        chain_hasher.update(comp_hash);
        self.digest = chain_hasher.finalize().into();
        self.component_count += 1;
    }

    /// Return the current measurement digest.
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Return the number of components that have been extended into the chain.
    pub fn component_count(&self) -> usize {
        self.component_count
    }

    /// Return `true` if no components have been extended yet.
    pub fn is_empty(&self) -> bool {
        self.component_count == 0
    }
}

impl Default for MeasurementChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chain_is_empty_and_zero() {
        let chain = MeasurementChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.component_count(), 0);
        assert_eq!(chain.digest(), [0u8; 32]);
    }

    #[test]
    fn extend_changes_digest() {
        let mut chain = MeasurementChain::new();
        chain.extend(b"component-a");
        assert_ne!(chain.digest(), [0u8; 32]);
    }

    #[test]
    fn same_components_same_order_produce_same_digest() {
        let mut a = MeasurementChain::new();
        a.extend(b"bootloader:v1");
        a.extend(b"kernel:v5.15");

        let mut b = MeasurementChain::new();
        b.extend(b"bootloader:v1");
        b.extend(b"kernel:v5.15");

        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn different_order_produces_different_digest() {
        let mut a = MeasurementChain::new();
        a.extend(b"A");
        a.extend(b"B");

        let mut b = MeasurementChain::new();
        b.extend(b"B");
        b.extend(b"A");

        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn omitting_component_changes_digest() {
        let mut full = MeasurementChain::new();
        full.extend(b"A");
        full.extend(b"B");
        full.extend(b"C");

        let mut partial = MeasurementChain::new();
        partial.extend(b"A");
        partial.extend(b"C");

        assert_ne!(full.digest(), partial.digest());
    }

    #[test]
    fn component_count_is_tracked() {
        let mut chain = MeasurementChain::new();
        assert_eq!(chain.component_count(), 0);
        chain.extend(b"x");
        assert_eq!(chain.component_count(), 1);
        chain.extend(b"y");
        assert_eq!(chain.component_count(), 2);
    }
}
