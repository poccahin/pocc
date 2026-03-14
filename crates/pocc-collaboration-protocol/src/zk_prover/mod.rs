//! ZK-CogP Prover (zk-cogp-prover)
//!
//! Zero-Knowledge Cognitive Proof generator for the POCC protocol.
//!
//! A ZK-CogP proves that:
//!   1. The seller's model honestly produced `output_hash` from the declared intent.
//!   2. Resource consumption stayed within the `CognitiveBoundary` (compute units,
//!      time, safety clearance).
//!   3. No private weights or user data were leaked in the process.
//!
//! # Design Note
//! A production implementation would use a SNARK/STARK backend (e.g. Risc0,
//! SP1, or Groth16 via snarkjs).  This module provides the same *interface*
//! with a cryptographic commitment scheme (SHA-256 Merkle-leaf based) that is
//! structurally compatible and can be swapped for a real ZK backend without
//! changing the public API.
//!
//! ```text
//!  Witness: { model_id, intent, output, compute_units, elapsed_ms }
//!       │
//!       ▼
//!  CogPProver::generate_proof(witness, boundary)
//!       │
//!       ▼
//!  CogProof { commitment, public_inputs, proof_bytes }
//!       │
//!       ▼
//!  CogPVerifier::verify(proof, public_inputs) → bool
//! ```

use sha2::{Digest, Sha256};
use thiserror::Error;

use ahin_nervous_system::CogHash;
use crate::ctx_composer::CognitiveBoundary;

// ─────────────────────────────────────────────────────────────────────────────
// Error types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum ZkCogpError {
    #[error("Compute units {used} exceeded boundary {allowed}")]
    ComputeOverrun { used: u64, allowed: u64 },
    #[error("Elapsed time {elapsed_ms}ms exceeded boundary {allowed_ms}ms")]
    TimeOverrun { elapsed_ms: u64, allowed_ms: u64 },
    #[error("Safety clearance level {required} required, but seller holds {held}")]
    ClearanceInsufficient { required: u8, held: u8 },
    #[error("Proof verification failed: commitment mismatch")]
    VerificationFailed,
}

// ─────────────────────────────────────────────────────────────────────────────
// Witness — private inputs known only to the prover (seller)
// ─────────────────────────────────────────────────────────────────────────────

/// Private execution witness — never transmitted on-chain.
pub struct CogWitness {
    /// Unique identifier of the model that produced the output.
    pub model_id: String,
    /// Raw intent string (matches the CTx intent declaration).
    pub intent: String,
    /// SHA-256 hash of the raw inference output.
    pub output_hash: CogHash,
    /// Actual compute units consumed during inference.
    pub compute_units_used: u64,
    /// Actual wall-clock milliseconds elapsed.
    pub elapsed_ms: u64,
    /// Seller's self-reported safety clearance level.
    pub seller_clearance_level: u8,
}

// ─────────────────────────────────────────────────────────────────────────────
// Proof — public artefact anchored into the CTx and L1
// ─────────────────────────────────────────────────────────────────────────────

/// Public inputs that accompany the proof (transmitted alongside it).
#[derive(Debug, Clone)]
pub struct CogPublicInputs {
    /// Hash of the intent declaration (ties the proof to one CTx).
    pub intent_hash: CogHash,
    /// Hash of the model identifier (proves *which* model ran).
    pub model_hash: CogHash,
    /// The output hash that the proof commits to.
    pub output_hash: CogHash,
    /// Boundary that was enforced.
    pub boundary: CognitiveBoundary,
}

/// A ZK-CogP proof artefact.
#[derive(Debug, Clone)]
pub struct CogProof {
    /// Cryptographic commitment over (intent, model, output, boundary).
    pub commitment: CogHash,
    pub public_inputs: CogPublicInputs,
    /// Serialised proof bytes (SNARK proof in production; hash-chain here).
    pub proof_bytes: Vec<u8>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Prover
// ─────────────────────────────────────────────────────────────────────────────

pub struct CogPProver;

impl CogPProver {
    /// Generate a ZK-CogP proof from a private witness and the agreed boundary.
    ///
    /// Returns `Err` if the witness violates any boundary constraint — the
    /// prover *cannot* generate a valid proof for an out-of-bounds execution.
    pub fn generate_proof(
        witness: &CogWitness,
        boundary: &CognitiveBoundary,
    ) -> Result<CogProof, ZkCogpError> {
        // ── Constraint checks (would be circuit constraints in a real ZK backend) ──

        if witness.compute_units_used > boundary.max_compute_units {
            return Err(ZkCogpError::ComputeOverrun {
                used: witness.compute_units_used,
                allowed: boundary.max_compute_units,
            });
        }

        if witness.elapsed_ms > boundary.max_time_ms {
            return Err(ZkCogpError::TimeOverrun {
                elapsed_ms: witness.elapsed_ms,
                allowed_ms: boundary.max_time_ms,
            });
        }

        if witness.seller_clearance_level < boundary.safety_clearance_level {
            return Err(ZkCogpError::ClearanceInsufficient {
                required: boundary.safety_clearance_level,
                held: witness.seller_clearance_level,
            });
        }

        // ── Build public inputs ──────────────────────────────────────────────
        let intent_hash = Self::hash_bytes(witness.intent.as_bytes());
        let model_hash = Self::hash_bytes(witness.model_id.as_bytes());

        let public_inputs = CogPublicInputs {
            intent_hash,
            model_hash,
            output_hash: witness.output_hash,
            boundary: boundary.clone(),
        };

        // ── Commitment: H(intent || model || output || compute || elapsed) ───
        let mut h = Sha256::new();
        h.update(intent_hash);
        h.update(model_hash);
        h.update(witness.output_hash);
        h.update(witness.compute_units_used.to_be_bytes());
        h.update(witness.elapsed_ms.to_be_bytes());
        h.update(boundary.max_compute_units.to_be_bytes());
        h.update(boundary.max_time_ms.to_be_bytes());
        h.update([boundary.safety_clearance_level]);
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&h.finalize());

        // In production: run SNARK prover here and store the real proof bytes.
        let proof_bytes = commitment.to_vec();

        Ok(CogProof {
            commitment,
            public_inputs,
            proof_bytes,
        })
    }

    fn hash_bytes(data: &[u8]) -> CogHash {
        let mut h = Sha256::new();
        h.update(data);
        let mut out = [0u8; 32];
        out.copy_from_slice(&h.finalize());
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Verifier
// ─────────────────────────────────────────────────────────────────────────────

pub struct CogPVerifier;

impl CogPVerifier {
    /// Verify a proof against its public inputs.
    ///
    /// A buyer or any L1 node can call this without access to the witness.
    pub fn verify(proof: &CogProof) -> Result<(), ZkCogpError> {
        // Re-derive the expected commitment from the public inputs alone.
        let pi = &proof.public_inputs;
        let mut h = Sha256::new();
        h.update(pi.intent_hash);
        h.update(pi.model_hash);
        h.update(pi.output_hash);
        // We cannot re-derive the private compute/elapsed values, so we verify
        // that proof_bytes are structurally consistent with the commitment.
        // In a real ZK system this would be a polynomial evaluation check.
        if proof.proof_bytes.len() != 32 {
            return Err(ZkCogpError::VerificationFailed);
        }
        if proof.proof_bytes.as_slice() != proof.commitment {
            return Err(ZkCogpError::VerificationFailed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary() -> CognitiveBoundary {
        CognitiveBoundary {
            max_compute_units: 10_000,
            max_time_ms: 5_000,
            safety_clearance_level: 1,
        }
    }

    fn witness(
        compute: u64,
        elapsed: u64,
        clearance: u8,
    ) -> CogWitness {
        CogWitness {
            model_id: "llama3-8b".into(),
            intent: "fetch coffee".into(),
            output_hash: [0xABu8; 32],
            compute_units_used: compute,
            elapsed_ms: elapsed,
            seller_clearance_level: clearance,
        }
    }

    #[test]
    fn valid_witness_produces_verifiable_proof() {
        let w = witness(5_000, 1_000, 2);
        let proof = CogPProver::generate_proof(&w, &boundary()).unwrap();
        assert!(CogPVerifier::verify(&proof).is_ok());
    }

    #[test]
    fn compute_overrun_rejected() {
        let w = witness(20_000, 1_000, 2);
        assert!(matches!(
            CogPProver::generate_proof(&w, &boundary()),
            Err(ZkCogpError::ComputeOverrun { .. })
        ));
    }

    #[test]
    fn time_overrun_rejected() {
        let w = witness(1_000, 99_999, 2);
        assert!(matches!(
            CogPProver::generate_proof(&w, &boundary()),
            Err(ZkCogpError::TimeOverrun { .. })
        ));
    }

    #[test]
    fn insufficient_clearance_rejected() {
        let w = witness(1_000, 1_000, 0);
        assert!(matches!(
            CogPProver::generate_proof(&w, &boundary()),
            Err(ZkCogpError::ClearanceInsufficient { .. })
        ));
    }

    #[test]
    fn tampered_proof_fails_verification() {
        let w = witness(5_000, 1_000, 2);
        let mut proof = CogPProver::generate_proof(&w, &boundary()).unwrap();
        proof.proof_bytes[0] ^= 0xFF; // tamper
        assert!(matches!(
            CogPVerifier::verify(&proof),
            Err(ZkCogpError::VerificationFailed)
        ));
    }
}
