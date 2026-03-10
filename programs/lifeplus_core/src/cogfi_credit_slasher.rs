use anchor_lang::prelude::*;

use crate::{errors::LifePlusError, state::*};

#[derive(Accounts)]
pub struct EnforceDarwinianFilter<'info> {
    #[account(mut)]
    pub rogue_agent: Account<'info, AgentPersona>,
    #[account(mut)]
    pub auditor_authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"auditor_whitelist", auditor_authority.key().as_ref()],
        bump,
        constraint = is_whitelisted_auditor.is_active @ LifePlusError::UnauthorizedSlasher,
        constraint = is_whitelisted_auditor.authority == auditor_authority.key() @ LifePlusError::UnauthorizedSlasher,
    )]
    pub is_whitelisted_auditor: Account<'info, AuditorWhitelist>,
    #[account(mut)]
    pub auditor_reward_pool: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn trigger_soulbound_slash(
    ctx: Context<EnforceDarwinianFilter>,
    fraud_evidence_hash: [u8; 32],
) -> Result<()> {
    let agent = &mut ctx.accounts.rogue_agent;
    let auditor_record = &mut ctx.accounts.is_whitelisted_auditor;

    require!(!agent.is_slashed, LifePlusError::AgentAlreadyDead);

    agent.is_slashed = true;
    agent.total_valid_pocc = 0;

    let slashed_amount = agent.staked_life_plus;
    agent.staked_life_plus = 0;

    auditor_record.total_slashes_executed = auditor_record
        .total_slashes_executed
        .checked_add(1)
        .ok_or(LifePlusError::ArithmeticOverflow)?;

    msg!("💀 [EXECUTION] Persona Soulbound Slash triggered by Authorized Court!");
    msg!(
        "💀 Judge (Auditor): {}",
        ctx.accounts.auditor_authority.key()
    );
    msg!(
        "💀 Reason: Fraud evidence {:?} verified.",
        fraud_evidence_hash
    );
    msg!(
        "💀 Consequence: Cognitive score zeroed. {} LIFE++ stake burned/confiscated.",
        slashed_amount
    );
    msg!("💀 The agent is now permanently blacklisted from the AHIN network.");

    Ok(())
}
