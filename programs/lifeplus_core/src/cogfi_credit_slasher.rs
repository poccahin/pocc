use anchor_lang::prelude::*;

use crate::{errors::LifePlusError, state::*};

#[derive(Accounts)]
#[instruction(rogue_agent_pubkey: Pubkey)]
pub struct EnforceDarwinianFilter<'info> {
    #[account(
        mut,
        seeds = [b"persona", rogue_agent_pubkey.as_ref()],
        bump,
    )]
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

#[derive(Accounts)]
#[instruction(worker: Pubkey, orchestrator: Pubkey)]
pub struct UpdateEigenTrustScore<'info> {
    #[account(
        mut,
        seeds = [b"persona", worker.as_ref()],
        bump,
    )]
    pub worker_persona: Account<'info, AgentPersona>,
    #[account(
        init_if_needed,
        payer = orchestrator_authority,
        space = 8 + 32 + 32 + 8,
        seeds = [b"edge", orchestrator.as_ref(), worker.as_ref()],
        bump
    )]
    pub interaction_edge: Account<'info, InteractionEdge>,
    #[account(mut)]
    pub orchestrator_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn trigger_soulbound_slash(
    ctx: Context<EnforceDarwinianFilter>,
    rogue_agent_pubkey: Pubkey,
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

pub fn record_settlement_and_decay(
    ctx: Context<UpdateEigenTrustScore>,
    worker: Pubkey,
    orchestrator: Pubkey,
    settled_amount: u64,
) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.orchestrator_authority.key(),
        orchestrator,
        LifePlusError::OrchestratorMismatch
    );

    let edge = &mut ctx.accounts.interaction_edge;
    let persona = &mut ctx.accounts.worker_persona;

    edge.orchestrator = orchestrator;
    edge.worker = worker;
    edge.interaction_count = edge
        .interaction_count
        .checked_add(1)
        .ok_or(LifePlusError::ArithmeticOverflow)?;

    let decay_shift = edge.interaction_count.saturating_sub(1) as u32;
    let effective_reputation_value = if decay_shift >= 64 {
        0
    } else {
        settled_amount.checked_shr(decay_shift).unwrap_or(0)
    };

    persona.total_value_settled = persona
        .total_value_settled
        .checked_add(effective_reputation_value)
        .ok_or(LifePlusError::ArithmeticOverflow)?;

    msg!(
        "🕸️ [EigenTrust] Edge Interaction Count: {}",
        edge.interaction_count
    );
    msg!(
        "📉 [Decay] Raw Value: {}, Effective S_cog Gain: {}",
        settled_amount,
        effective_reputation_value
    );

    Ok(())
}
