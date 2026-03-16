//! Tamper-evident append-only audit log for the key vault.
//!
//! Every signing event, authorization failure, and enrollment is recorded as an
//! [`AuditEntry`].  Entries are chained in a SHA-256 Merkle-style hash chain:
//!
//! ```text
//! entry[0]: prev_hash = 0x000…0,  hash = SHA-256(0x000…0 ‖ payload[0])
//! entry[1]: prev_hash = hash[0],  hash = SHA-256(hash[0] ‖ payload[1])
//! entry[N]: prev_hash = hash[N-1], hash = SHA-256(hash[N-1] ‖ payload[N])
//! ```
//!
//! Any retrospective modification of an earlier entry invalidates all
//! subsequent hashes, making tampering immediately detectable.

use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single entry in the vault audit log.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// UNIX timestamp (seconds) at which the event was recorded.
    pub timestamp: u64,
    /// Human-readable description of the event.
    pub event: String,
    /// The agent DID involved (empty string for system events).
    pub agent_id: String,
    /// SHA-256 of the previous entry's hash (genesis entry uses `[0u8; 32]`).
    pub prev_hash: [u8; 32],
    /// SHA-256(prev_hash ‖ timestamp_be ‖ event ‖ agent_id).
    pub hash: [u8; 32],
}

impl AuditEntry {
    /// Build a new entry, chaining it to `prev_hash`.
    pub(crate) fn new(event: String, agent_id: String, prev_hash: [u8; 32]) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let hash = Self::compute_hash(&prev_hash, timestamp, &event, &agent_id);
        Self {
            timestamp,
            event,
            agent_id,
            prev_hash,
            hash,
        }
    }

    fn compute_hash(prev: &[u8; 32], ts: u64, event: &str, agent_id: &str) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"audit:v1:");
        h.update(prev);
        h.update(ts.to_be_bytes());
        h.update(event.as_bytes());
        h.update(b":");
        h.update(agent_id.as_bytes());
        h.finalize().into()
    }
}

/// An append-only, hash-chained audit log.
///
/// The log is held in memory for the lifetime of the vault.  In production it
/// should be persisted to sealed storage and flushed to an external audit
/// service over an authenticated channel.
#[derive(Debug, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    /// Create an empty audit log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a new event to the log, chained to the previous entry's hash.
    pub fn append(&mut self, event: impl Into<String>, agent_id: impl Into<String>) {
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.hash)
            .unwrap_or([0u8; 32]);
        self.entries.push(AuditEntry::new(event.into(), agent_id.into(), prev_hash));
    }

    /// Return all audit entries.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Verify the hash chain integrity from genesis to tip.
    ///
    /// Returns `true` if the chain is intact, `false` if any entry has been
    /// tampered with.
    pub fn verify_chain(&self) -> bool {
        let mut expected_prev = [0u8; 32];
        for entry in &self.entries {
            if entry.prev_hash != expected_prev {
                return false;
            }
            let recomputed = AuditEntry::compute_hash(
                &entry.prev_hash,
                entry.timestamp,
                &entry.event,
                &entry.agent_id,
            );
            if recomputed != entry.hash {
                return false;
            }
            expected_prev = entry.hash;
        }
        true
    }

    /// Return the number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return `true` if the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_log_chain_is_valid() {
        let log = AuditLog::new();
        assert!(log.verify_chain());
    }

    #[test]
    fn single_entry_chain_is_valid() {
        let mut log = AuditLog::new();
        log.append("vault_initialized", "");
        assert!(log.verify_chain());
    }

    #[test]
    fn multi_entry_chain_is_valid() {
        let mut log = AuditLog::new();
        log.append("vault_initialized", "");
        log.append("agent_enrolled", "did:ahin:agent:alice");
        log.append("signing_request", "did:ahin:agent:alice");
        log.append("signing_success", "did:ahin:agent:alice");
        assert!(log.verify_chain());
        assert_eq!(log.len(), 4);
    }

    #[test]
    fn tampered_event_breaks_chain() {
        let mut log = AuditLog::new();
        log.append("vault_initialized", "");
        log.append("agent_enrolled", "did:ahin:agent:alice");
        // Tamper with the first entry's event field
        log.entries[0].event = "vault_pwned".to_string();
        assert!(!log.verify_chain());
    }

    #[test]
    fn tampered_hash_breaks_chain() {
        let mut log = AuditLog::new();
        log.append("vault_initialized", "");
        log.append("agent_enrolled", "did:ahin:agent:alice");
        // Corrupt the hash of the first entry (affects second entry's prev_hash check)
        log.entries[0].hash[0] ^= 0xFF;
        assert!(!log.verify_chain());
    }
}
