use anchor_lang::prelude::*;

pub mod cogfi_credit_slasher;
pub mod daily_netting_clearing;
pub mod composite_ctx_settlement;
pub mod errors;
pub mod pocc_structural_verifier;
pub mod state;

use cogfi_credit_slasher::*;
use daily_netting_clearing::*;
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
    ) -> Result<()> {
        composite_ctx_settlement::process_x402_micro_settlement(ctx, subtask_id)
    }


    pub fn record_settlement_and_decay(
        ctx: Context<UpdateEigenTrustScore>,
        worker: Pubkey,
        orchestrator: Pubkey,
        settled_amount: u64,
    ) -> Result<()> {
        cogfi_credit_slasher::record_settlement_and_decay(
            ctx,
            worker,
            orchestrator,
            settled_amount,
        )
    }

    pub fn trigger_soulbound_slash(
        ctx: Context<EnforceDarwinianFilter>,
        fraud_evidence_hash: [u8; 32],
    ) -> Result<()> {
        cogfi_credit_slasher::trigger_soulbound_slash(ctx, fraud_evidence_hash)
    }

    pub fn submit_daily_netting_with_da(
        ctx: Context<SubmitDailyNetting>,
        epoch_id: u64,
        merkle_root: [u8; 32],
        da_blob_id: [u8; 32],
    ) -> Result<()> {
        daily_netting_clearing::submit_daily_netting_with_da(ctx, epoch_id, merkle_root, da_blob_id)
    }

    pub fn finalize_netting_epoch(ctx: Context<FinalizeNettingEpoch>) -> Result<()> {
        daily_netting_clearing::finalize_netting_epoch(ctx)
    }

    pub fn slash_for_data_withholding(ctx: Context<SlashForDataWithholding>) -> Result<()> {
        daily_netting_clearing::slash_for_data_withholding(ctx)
    }
}
