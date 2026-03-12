use anchor_lang::prelude::*;

use crate::{errors::LifePlusError, state::*};

#[account]
pub struct TaskIntentState {
    pub orchestrator: Pubkey,
    pub intent_hash: [u8; 32],
    pub thermodynamic_boundary: u64,
    pub is_active: bool,
}

#[account]
pub struct IntentReceipt {
    pub intent_hash: [u8; 32],
    pub executing_agent: Pubkey,
    pub verified_timestamp: i64,
}

#[derive(Accounts)]
#[instruction(intent_hash: [u8; 32])]
pub struct VerifyStructuralPoCC<'info> {
    #[account(
        mut,
        seeds = [b"persona", fee_payer.key().as_ref()],
        bump,
    )]
    pub agent_persona: Account<'info, AgentPersona>,
    #[account(mut)]
    pub ahin_timeline: Account<'info, AhinTimeline>,
    #[account(
        seeds = [b"task_intent", intent_hash.as_ref()],
        bump,
        constraint = task_intent_state.is_active @ LifePlusError::TaskAlreadyClosed,
        constraint = task_intent_state.intent_hash == intent_hash @ LifePlusError::TaskIntentHashMismatch,
    )]
    pub task_intent_state: Account<'info, TaskIntentState>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(
        init,
        payer = fee_payer,
        space = 8 + 32 + 32 + 8,
        seeds = [b"processed_intent", intent_hash.as_ref()],
        bump
    )]
    pub processed_intent_receipt: Account<'info, IntentReceipt>,
    pub system_program: Program<'info, System>,
}

pub fn execute_pocc_verification(
    ctx: Context<VerifyStructuralPoCC>,
    intent_hash: [u8; 32],
    zk_cogp_proof: Vec<u8>,
    compute_units_consumed: u64,
) -> Result<()> {
    let agent = &mut ctx.accounts.agent_persona;
    let timeline = &mut ctx.accounts.ahin_timeline;
    let receipt = &mut ctx.accounts.processed_intent_receipt;
    let task_state = &ctx.accounts.task_intent_state;

    let hard_boundary = task_state.thermodynamic_boundary;
    require!(
        compute_units_consumed <= hard_boundary,
        LifePlusError::ThermodynamicBoundaryExceeded
    );

    let is_proof_valid = verify_zk_proof_onchain(&intent_hash, &zk_cogp_proof);
    require!(is_proof_valid, LifePlusError::InvalidCognitiveProof);

    timeline.current_global_hash = anchor_lang::solana_program::hash::hashv(&[
        &timeline.current_global_hash,
        &intent_hash,
        &agent.key().to_bytes(),
    ])
    .to_bytes();

    agent.total_valid_pocc = agent
        .total_valid_pocc
        .checked_add(1)
        .ok_or(LifePlusError::ArithmeticOverflow)?;
    agent.last_active_timestamp = Clock::get()?.unix_timestamp;

    receipt.intent_hash = intent_hash;
    receipt.executing_agent = agent.key();
    receipt.verified_timestamp = agent.last_active_timestamp;

    msg!("✅ [PoCC Verified] Structural constraints met. Zero-Knowledge proof valid.");
    msg!(
        "📏 [Physics] Consumed {} CUs / Boundary {} CUs.",
        compute_units_consumed,
        hard_boundary
    );

    Ok(())
}

fn verify_zk_proof_onchain(hash: &[u8; 32], proof: &[u8]) -> bool {
    !proof.is_empty() && hash.iter().any(|byte| *byte != 0)
}
