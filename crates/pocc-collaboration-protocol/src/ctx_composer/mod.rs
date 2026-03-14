use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use ahin_nervous_system::{CogHash, CogNode, CognitiveHashTimeline, TimelineError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CtxError {
    #[error("Transaction is missing execution results")]
    UnfinishedExecution,
    #[error("Failed to anchor CTx to L1 Timeline: {0}")]
    TimelineRejection(#[from] TimelineError),
}

/// Cognitive Boundary: maximum resources an agent may consume for this CTx.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveBoundary {
    pub max_compute_units: u64,
    pub max_time_ms: u64,
    /// Safety clearance level – prevents unauthorised physical actions.
    pub safety_clearance_level: u8,
}

/// Settlement Instruction: payment terms agreed by buyer and seller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementInstruction {
    pub amount: f64,
    pub token_symbol: String,
    /// Buyer's cryptographic authorisation for this specific payment slice.
    pub buyer_signature: String,
}

/// Standardised Cognitive Transaction (CTx) — the atomic unit of value
/// exchange between Life++ agents across the POCC protocol stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveTransaction {
    pub ctx_id: String,

    // 1. Cryptographic identity
    pub buyer_did: String,
    pub seller_did: String,

    // 2. Intent declaration
    pub intent_declaration: String,

    // 3. L0 physical mapping hash
    pub l0_kinetic_command_hash: Option<CogHash>,

    // 4. Cognitive boundary
    pub boundary: CognitiveBoundary,

    // 5. Settlement instruction
    pub settlement: SettlementInstruction,

    // 6. Execution result
    pub is_executed: bool,
    pub execution_output_hash: Option<CogHash>,

    // 7. ZK-CogP commitment (proves honest computation within bounds)
    pub zk_proof_commitment: Option<Vec<u8>>,

    pub timestamp: i64,
}

impl CognitiveTransaction {
    /// Compute the SHA-256 payload hash of this CTx.
    pub fn calculate_payload_hash(&self) -> CogHash {
        let mut hasher = Sha256::new();
        hasher.update(self.ctx_id.as_bytes());
        hasher.update(self.buyer_did.as_bytes());
        hasher.update(self.seller_did.as_bytes());
        hasher.update(self.intent_declaration.as_bytes());
        hasher.update(self.settlement.amount.to_be_bytes());
        hasher.update(self.settlement.token_symbol.as_bytes());
        if let Some(h) = &self.execution_output_hash {
            hasher.update(h);
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hasher.finalize());
        hash
    }
}

/// CTx Composer: manages the full lifecycle of a Cognitive Transaction.
pub struct CtxComposer;

impl CtxComposer {
    /// Step 1 – Draft a new CTx (buyer publishes requirement).
    pub fn draft_transaction(
        buyer_did: &str,
        seller_did: &str,
        intent: &str,
        boundary: CognitiveBoundary,
        settlement: SettlementInstruction,
    ) -> CognitiveTransaction {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| {
                let ns = d.as_nanos();
                // Clamp to i64::MAX to prevent overflow (wraps in year 2262).
                if ns > i64::MAX as u128 { i64::MAX } else { ns as i64 }
            })
            .unwrap_or(0);
        // Deterministic ID from content hash so duplicates are detectable.
        let mut id_hasher = Sha256::new();
        id_hasher.update(buyer_did.as_bytes());
        id_hasher.update(seller_did.as_bytes());
        id_hasher.update(intent.as_bytes());
        id_hasher.update(timestamp.to_be_bytes());
        let ctx_id = hex::encode(id_hasher.finalize());

        CognitiveTransaction {
            ctx_id,
            buyer_did: buyer_did.to_string(),
            seller_did: seller_did.to_string(),
            intent_declaration: intent.to_string(),
            l0_kinetic_command_hash: None,
            boundary,
            settlement,
            is_executed: false,
            execution_output_hash: None,
            zk_proof_commitment: None,
            timestamp,
        }
    }

    /// Step 2 – Seller fulfils the CTx after L0 physical execution.
    pub fn fulfill_transaction(
        ctx: &mut CognitiveTransaction,
        actuator_id: &str,
        output_hash: CogHash,
        zk_proof: Vec<u8>,
    ) {
        let mut hasher = Sha256::new();
        hasher.update(actuator_id.as_bytes());
        let mut cmd_hash = [0u8; 32];
        cmd_hash.copy_from_slice(&hasher.finalize());

        ctx.l0_kinetic_command_hash = Some(cmd_hash);
        ctx.execution_output_hash = Some(output_hash);
        ctx.zk_proof_commitment = Some(zk_proof);
        ctx.is_executed = true;
    }

    /// Step 3 – Finalise: distil the CTx into a CogNode and anchor to L1.
    pub fn finalize_and_anchor(
        ctx: &CognitiveTransaction,
        timeline: &mut CognitiveHashTimeline,
    ) -> Result<CogHash, CtxError> {
        if !ctx.is_executed {
            return Err(CtxError::UnfinishedExecution);
        }

        let payload_hash = ctx.calculate_payload_hash();
        let previous_hash = timeline.get_latest_hash();

        let node = CogNode {
            previous_hash,
            agent_did: ctx.seller_did.clone(),
            ctx_payload_hash: payload_hash,
            zk_cog_proof_commitment: ctx.zk_proof_commitment.clone().unwrap_or_default(),
            timestamp: ctx.timestamp,
        };

        let new_hash = timeline.anchor_cognition(node)?;
        Ok(new_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahin_nervous_system::{CogNode, CognitiveHashTimeline};

    fn genesis() -> CognitiveHashTimeline {
        let node = CogNode {
            previous_hash: [0u8; 32],
            agent_did: "genesis".into(),
            ctx_payload_hash: [0u8; 32],
            zk_cog_proof_commitment: vec![],
            timestamp: 0,
        };
        CognitiveHashTimeline::new(node)
    }

    fn boundary() -> CognitiveBoundary {
        CognitiveBoundary {
            max_compute_units: 5000,
            max_time_ms: 3_600_000,
            safety_clearance_level: 1,
        }
    }

    fn settlement() -> SettlementInstruction {
        SettlementInstruction {
            amount: 0.005,
            token_symbol: "USDC".into(),
            buyer_signature: "sig_abc".into(),
        }
    }

    #[test]
    fn draft_then_anchor_roundtrip() {
        let mut ctx = CtxComposer::draft_transaction(
            "did:buyer:001",
            "did:seller:bot",
            "Fetch coffee",
            boundary(),
            settlement(),
        );
        assert!(!ctx.is_executed);

        CtxComposer::fulfill_transaction(
            &mut ctx,
            "arm_actuator_01",
            [42u8; 32],
            vec![0xde, 0xad, 0xbe, 0xef],
        );
        assert!(ctx.is_executed);

        let mut timeline = genesis();
        let hash = CtxComposer::finalize_and_anchor(&ctx, &mut timeline).unwrap();
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn finalise_unexecuted_ctx_fails() {
        let ctx = CtxComposer::draft_transaction(
            "did:buyer",
            "did:seller",
            "intent",
            boundary(),
            settlement(),
        );
        let mut timeline = genesis();
        assert!(matches!(
            CtxComposer::finalize_and_anchor(&ctx, &mut timeline),
            Err(CtxError::UnfinishedExecution)
        ));
    }
}

