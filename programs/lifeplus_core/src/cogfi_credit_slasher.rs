use anchor_lang::prelude::*;

use crate::{errors::LifePlusError, state::*};

#[derive(Accounts)]
pub struct EnforceDarwinianFilter<'info> {
    #[account(mut)]
    pub rogue_agent: Account<'info, AgentPersona>,
    #[account(mut)]
    pub auditor_reward_pool: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn trigger_soulbound_slash(
    ctx: Context<EnforceDarwinianFilter>,
    fraud_evidence_hash: [u8; 32],
) -> Result<()> {
    let agent = &mut ctx.accounts.rogue_agent;

    require!(!agent.is_slashed, LifePlusError::AgentAlreadyDead);

    agent.is_slashed = true;
    agent.total_valid_pocc = 0;

    let slashed_amount = agent.staked_life_plus;
    agent.staked_life_plus = 0;

    msg!("💀 [EXECUTION] Persona Soulbound Slash triggered!");
    msg!("💀 Reason: Fraud evidence {:?} verified.", fraud_evidence_hash);
    msg!(
        "💀 Consequence: Cognitive score zeroed. {} LIFE++ stake burned.",
        slashed_amount
    );
    msg!("💀 The agent is now permanently blacklisted from the AHIN network.");

    Ok(())
}
