use anchor_lang::prelude::*;

pub mod cogfi_credit_slasher;
pub mod composite_ctx_settlement;
pub mod errors;
pub mod pocc_structural_verifier;
pub mod state;

use cogfi_credit_slasher::*;
use composite_ctx_settlement::*;
use pocc_structural_verifier::*;

declare_id!("LifeP1usCore1111111111111111111111111111111");

#[program]
pub mod lifeplus_core {
    use super::*;

    pub fn execute_pocc_verification(
        ctx: Context<VerifyStructuralPoCC>,
        intent_hash: [u8; 32],
        zk_cogp_proof: Vec<u8>,
        compute_units_consumed: u64,
    ) -> Result<()> {
        pocc_structural_verifier::execute_pocc_verification(
            ctx,
            intent_hash,
            zk_cogp_proof,
            compute_units_consumed,
        )
    }

    pub fn process_x402_micro_settlement(
        ctx: Context<SettleWorkerCTx>,
        subtask_id: u64,
        payment_bips: u16,
    ) -> Result<()> {
        composite_ctx_settlement::process_x402_micro_settlement(ctx, subtask_id, payment_bips)
    }

    pub fn trigger_soulbound_slash(
        ctx: Context<EnforceDarwinianFilter>,
        fraud_evidence_hash: [u8; 32],
    ) -> Result<()> {
        cogfi_credit_slasher::trigger_soulbound_slash(ctx, fraud_evidence_hash)
    }
}
