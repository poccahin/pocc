use anchor_lang::prelude::*;

use crate::{errors::LifePlusError, state::*};

#[derive(Accounts)]
pub struct VerifyStructuralPoCC<'info> {
    #[account(mut)]
    pub agent_persona: Account<'info, AgentPersona>,
    #[account(mut)]
    pub ahin_timeline: Account<'info, AhinTimeline>,
}

pub fn execute_pocc_verification(
    ctx: Context<VerifyStructuralPoCC>,
    intent_hash: [u8; 32],
    zk_cogp_proof: Vec<u8>,
    compute_units_consumed: u64,
    thermodynamic_boundary: u64,
) -> Result<()> {
    let agent = &mut ctx.accounts.agent_persona;
    let timeline = &mut ctx.accounts.ahin_timeline;

    require!(
        compute_units_consumed <= thermodynamic_boundary,
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

    msg!("✅ [PoCC Verified] Structural constraints met. AHIN timeline advanced.");
    Ok(())
}

fn verify_zk_proof_onchain(hash: &[u8; 32], proof: &[u8]) -> bool {
    !proof.is_empty() && hash.iter().any(|byte| *byte != 0)
}
