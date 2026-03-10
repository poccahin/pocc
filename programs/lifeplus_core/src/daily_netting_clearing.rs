use anchor_lang::prelude::*;

use crate::{errors::LifePlusError, state::*};

pub const CHALLENGE_WINDOW_SECONDS: i64 = 86_400;

#[account]
pub struct NettingEpoch {
    pub orchestrator: Pubkey,
    pub epoch_id: u64,
    pub merkle_root: [u8; 32],
    pub da_blob_id: [u8; 32],
    pub submission_timestamp: i64,
    pub is_finalized: bool,
}

impl NettingEpoch {
    pub const LEN: usize = 32 + 8 + 32 + 32 + 8 + 1;
}

#[derive(Accounts)]
#[instruction(epoch_id: u64)]
pub struct SubmitDailyNetting<'info> {
    #[account(mut)]
    pub orchestrator: Signer<'info>,
    #[account(
        init,
        payer = orchestrator,
        space = 8 + NettingEpoch::LEN,
        seeds = [b"netting_epoch", orchestrator.key().as_ref(), &epoch_id.to_le_bytes()],
        bump
    )]
    pub netting_epoch: Account<'info, NettingEpoch>,
    pub system_program: Program<'info, System>,
}

pub fn submit_daily_netting_with_da(
    ctx: Context<SubmitDailyNetting>,
    epoch_id: u64,
    merkle_root: [u8; 32],
    da_blob_id: [u8; 32],
) -> Result<()> {
    let epoch = &mut ctx.accounts.netting_epoch;

    require!(
        da_blob_id != [0; 32],
        LifePlusError::MissingDataAvailabilityBlob
    );

    epoch.orchestrator = ctx.accounts.orchestrator.key();
    epoch.epoch_id = epoch_id;
    epoch.merkle_root = merkle_root;
    epoch.da_blob_id = da_blob_id;
    epoch.submission_timestamp = Clock::get()?.unix_timestamp;
    epoch.is_finalized = false;

    msg!("📦 [Daily Netting] Epoch {} submitted by Orchestrator.", epoch_id);
    msg!(
        "🔗 [DA Anchored] Merkle Root stored. Arweave/Celestia Blob ID: {:?}",
        da_blob_id
    );
    msg!(
        "⏳ [Challenge Window] 24-hour Optimistic time-lock started. Awaiting DA verification."
    );

    Ok(())
}

#[derive(Accounts)]
pub struct FinalizeNettingEpoch<'info> {
    #[account(mut)]
    pub orchestrator: Signer<'info>,
    #[account(
        mut,
        has_one = orchestrator,
        constraint = !netting_epoch.is_finalized @ LifePlusError::EpochAlreadyFinalized,
    )]
    pub netting_epoch: Account<'info, NettingEpoch>,
}

pub fn finalize_netting_epoch(ctx: Context<FinalizeNettingEpoch>) -> Result<()> {
    let epoch = &mut ctx.accounts.netting_epoch;
    let current_time = Clock::get()?.unix_timestamp;
    let challenge_window_end = epoch.submission_timestamp + CHALLENGE_WINDOW_SECONDS;

    require!(
        current_time >= challenge_window_end,
        LifePlusError::ChallengeWindowExpired
    );

    epoch.is_finalized = true;

    msg!(
        "✅ [Epoch Finalized] Epoch {} finalized after challenge window.",
        epoch.epoch_id
    );

    Ok(())
}

#[derive(Accounts)]
pub struct SlashForDataWithholding<'info> {
    #[account(mut)]
    pub orchestrator_persona: Account<'info, AgentPersona>,
    #[account(
        mut,
        constraint = !netting_epoch.is_finalized @ LifePlusError::EpochAlreadyFinalized,
    )]
    pub netting_epoch: Account<'info, NettingEpoch>,
    #[account(mut)]
    pub auditor: Signer<'info>,
    #[account(
        mut,
        seeds = [b"auditor_whitelist", auditor.key().as_ref()],
        bump,
        constraint = is_whitelisted_auditor.is_active @ LifePlusError::UnauthorizedSlasher,
        constraint = is_whitelisted_auditor.authority == auditor.key() @ LifePlusError::UnauthorizedSlasher,
    )]
    pub is_whitelisted_auditor: Account<'info, AuditorWhitelist>,
}

pub fn slash_for_data_withholding(ctx: Context<SlashForDataWithholding>) -> Result<()> {
    let epoch = &mut ctx.accounts.netting_epoch;
    let persona = &mut ctx.accounts.orchestrator_persona;
    let current_time = Clock::get()?.unix_timestamp;
    let challenge_window_end = epoch.submission_timestamp + CHALLENGE_WINDOW_SECONDS;

    require!(
        current_time < challenge_window_end,
        LifePlusError::ChallengeWindowExpired
    );

    persona.is_slashed = true;
    persona.total_valid_pocc = 0;
    persona.staked_life_plus = 0;

    ctx.accounts.is_whitelisted_auditor.total_slashes_executed = ctx
        .accounts
        .is_whitelisted_auditor
        .total_slashes_executed
        .checked_add(1)
        .ok_or(LifePlusError::ArithmeticOverflow)?;

    epoch.is_finalized = true;

    msg!("💀 [EXECUTION] Data Withholding Detected! Orchestrator Slashed.");
    msg!("💀 The submitted DA Blob was a ghost payload. Trust nullified.");

    Ok(())
}
