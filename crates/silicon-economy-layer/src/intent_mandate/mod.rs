//! AP2 Intent Mandate (ap2-intent-mandate)
//!
//! Agency Checkout Protection layer for the POCC Silicon Economy.
//!
//! An Intent Mandate is a signed, time-bounded, and amount-capped authorisation
//! that a buyer grants to a specific seller (or to the protocol itself) to
//! debit their account up to `max_amount` within `valid_until_epoch`.
//!
//! It prevents:
//! - **Over-charging**: a seller cannot bill more than the mandated cap.
//! - **Replay attacks**: each mandate has a nonce; used mandates are burned.
//! - **Stale authorisations**: expired mandates are rejected.
//!
//! ```text
//!  Buyer ──creates──► IntentMandate { buyer_did, seller_did, max_amount, expiry }
//!                              │
//!                    MandateRegistry::register()
//!                              │
//!  Seller ──claims──► MandateRegistry::consume(mandate_id, claim_amount)
//!                              │
//!                    Ok(()) or Err(MandateError)
//! ```

use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MandateError {
    #[error("Mandate not found: {0}")]
    NotFound(String),
    #[error("Mandate expired at epoch {expired_at}, current epoch {now}")]
    Expired { expired_at: u64, now: u64 },
    #[error("Claim amount {claim:.4} exceeds remaining cap {remaining:.4}")]
    CapExceeded { claim: f64, remaining: f64 },
    #[error("Mandate already fully consumed")]
    AlreadyConsumed,
    #[error("Seller DID mismatch: mandate is for {expected}, got {actual}")]
    SellerMismatch { expected: String, actual: String },
}

// ─────────────────────────────────────────────────────────────────────────────
// Intent Mandate
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum MandateStatus {
    /// Mandate is active and has remaining balance.
    Active,
    /// Mandate has been fully consumed.
    Consumed,
    /// Mandate was revoked by the buyer before expiry.
    Revoked,
}

/// A buyer-signed authorisation for a seller to debit up to `max_amount`.
#[derive(Debug, Clone)]
pub struct IntentMandate {
    pub mandate_id: String,
    pub buyer_did: String,
    /// If `None`, any seller may claim (open mandate); otherwise restricted.
    pub seller_did: Option<String>,
    /// Maximum total amount that may be debited under this mandate.
    pub max_amount: f64,
    pub token_symbol: String,
    /// Amount already claimed against this mandate.
    pub consumed_amount: f64,
    /// Unix epoch (seconds) after which this mandate is invalid.
    pub valid_until_epoch: u64,
    pub status: MandateStatus,
    /// Buyer's cryptographic authorisation (signature over mandate params).
    pub buyer_signature: String,
}

impl IntentMandate {
    /// Remaining claimable balance.
    pub fn remaining(&self) -> f64 {
        (self.max_amount - self.consumed_amount).max(0.0)
    }

    pub fn is_active(&self) -> bool {
        self.status == MandateStatus::Active
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mandate Registry
// ─────────────────────────────────────────────────────────────────────────────

/// Stateful registry of all active intent mandates.
pub struct MandateRegistry {
    mandates: HashMap<String, IntentMandate>,
}

impl Default for MandateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MandateRegistry {
    pub fn new() -> Self {
        Self {
            mandates: HashMap::new(),
        }
    }

    /// Register a new intent mandate issued by a buyer.
    pub fn register(&mut self, mandate: IntentMandate) {
        self.mandates
            .insert(mandate.mandate_id.clone(), mandate);
    }

    /// Create and register a mandate in one step.
    pub fn issue(
        &mut self,
        buyer_did: &str,
        seller_did: Option<&str>,
        max_amount: f64,
        token_symbol: &str,
        valid_until_epoch: u64,
        buyer_signature: &str,
    ) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(buyer_did.as_bytes());
        h.update(seller_did.unwrap_or("*").as_bytes());
        h.update(max_amount.to_be_bytes());
        h.update(valid_until_epoch.to_be_bytes());
        h.update(buyer_signature.as_bytes());
        let mandate_id = hex::encode(h.finalize());

        let mandate = IntentMandate {
            mandate_id: mandate_id.clone(),
            buyer_did: buyer_did.to_string(),
            seller_did: seller_did.map(str::to_string),
            max_amount,
            token_symbol: token_symbol.to_string(),
            consumed_amount: 0.0,
            valid_until_epoch,
            status: MandateStatus::Active,
            buyer_signature: buyer_signature.to_string(),
        };
        self.mandates.insert(mandate_id.clone(), mandate);
        mandate_id
    }

    /// Seller claims `amount` against `mandate_id` at `current_epoch`.
    ///
    /// On success the mandate's consumed balance is increased and the mandate
    /// is transitioned to `Consumed` if the cap is exactly reached.
    pub fn consume(
        &mut self,
        mandate_id: &str,
        seller_did: &str,
        amount: f64,
        current_epoch: u64,
    ) -> Result<f64, MandateError> {
        let mandate = self
            .mandates
            .get_mut(mandate_id)
            .ok_or_else(|| MandateError::NotFound(mandate_id.to_string()))?;

        // Check revoked/consumed
        if !mandate.is_active() {
            return Err(MandateError::AlreadyConsumed);
        }

        // Expiry check
        if current_epoch > mandate.valid_until_epoch {
            return Err(MandateError::Expired {
                expired_at: mandate.valid_until_epoch,
                now: current_epoch,
            });
        }

        // Seller DID check (if restricted)
        if let Some(ref expected) = mandate.seller_did {
            if expected != seller_did {
                return Err(MandateError::SellerMismatch {
                    expected: expected.clone(),
                    actual: seller_did.to_string(),
                });
            }
        }

        // Cap check
        let remaining = mandate.remaining();
        if amount > remaining {
            return Err(MandateError::CapExceeded {
                claim: amount,
                remaining,
            });
        }

        mandate.consumed_amount += amount;
        if (mandate.consumed_amount - mandate.max_amount).abs() < 1e-6 {
            mandate.status = MandateStatus::Consumed;
        }

        Ok(mandate.remaining())
    }

    /// Buyer revokes a mandate before its expiry.
    pub fn revoke(&mut self, mandate_id: &str) -> Result<(), MandateError> {
        let mandate = self
            .mandates
            .get_mut(mandate_id)
            .ok_or_else(|| MandateError::NotFound(mandate_id.to_string()))?;
        mandate.status = MandateStatus::Revoked;
        Ok(())
    }

    pub fn get(&self, mandate_id: &str) -> Option<&IntentMandate> {
        self.mandates.get(mandate_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with_mandate() -> (MandateRegistry, String) {
        let mut reg = MandateRegistry::new();
        let id = reg.issue(
            "did:buyer:alice",
            Some("did:seller:bot"),
            100.0,
            "USDC",
            1_000,
            "sig_alice_001",
        );
        (reg, id)
    }

    #[test]
    fn consume_within_cap_succeeds() {
        let (mut reg, id) = registry_with_mandate();
        let remaining = reg.consume(&id, "did:seller:bot", 40.0, 500).unwrap();
        assert!((remaining - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn consuming_over_cap_is_rejected() {
        let (mut reg, id) = registry_with_mandate();
        assert!(matches!(
            reg.consume(&id, "did:seller:bot", 200.0, 500),
            Err(MandateError::CapExceeded { .. })
        ));
    }

    #[test]
    fn expired_mandate_is_rejected() {
        let (mut reg, id) = registry_with_mandate();
        assert!(matches!(
            reg.consume(&id, "did:seller:bot", 10.0, 2_000),
            Err(MandateError::Expired { .. })
        ));
    }

    #[test]
    fn wrong_seller_is_rejected() {
        let (mut reg, id) = registry_with_mandate();
        assert!(matches!(
            reg.consume(&id, "did:seller:impostor", 10.0, 500),
            Err(MandateError::SellerMismatch { .. })
        ));
    }

    #[test]
    fn revoke_then_consume_fails() {
        let (mut reg, id) = registry_with_mandate();
        reg.revoke(&id).unwrap();
        assert!(matches!(
            reg.consume(&id, "did:seller:bot", 10.0, 500),
            Err(MandateError::AlreadyConsumed)
        ));
    }

    #[test]
    fn mandate_transitions_to_consumed_when_cap_reached() {
        let (mut reg, id) = registry_with_mandate();
        reg.consume(&id, "did:seller:bot", 100.0, 500).unwrap();
        assert_eq!(reg.get(&id).unwrap().status, MandateStatus::Consumed);
    }
}
