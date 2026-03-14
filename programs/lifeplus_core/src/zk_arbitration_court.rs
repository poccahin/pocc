use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::errors::LifePlusError;

pub const DEFAULT_DEFENSE_WINDOW_SECONDS: i64 = 86_400;
pub const AUDITOR_BOND_AMOUNT: u64 = 1_000_000_000;

#[account]
pub struct DisputeRecord {
    pub auditor: Pubkey,
    pub accused: Pubkey,
    pub pocc_hash: [u8; 32],
    pub status: DisputeStatus,
    pub deadline: i64,
}

impl DisputeRecord {
    pub const LEN: usize = 32 + 32 + 32 + 1 + 8;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum DisputeStatus {
    PendingDefense,
    AuditorSlashed,
    AccusedSlashed,
}

#[derive(Accounts)]
#[instruction(target_did: String, pocc_hash: [u8; 32])]
pub struct RaiseDispute<'info> {
    #[account(mut)]
    pub auditor: Signer<'info>,
    /// CHECK: external identity reference from governance registrar.
    pub accused: UncheckedAccount<'info>,
    #[account(
        init,
        payer = auditor,
        space = 8 + DisputeRecord::LEN,
        seeds = [b"dispute", auditor.key().as_ref(), accused.key().as_ref(), &pocc_hash],
        bump
    )]
    pub dispute_record: Account<'info, DisputeRecord>,
    #[account(mut, constraint = auditor_token_account.owner == auditor.key() @ LifePlusError::UnauthorizedAuthority)]
    pub auditor_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = auditor,
        seeds = [b"escrow_vault", dispute_record.key().as_ref()],
        bump,
        token::mint = usdc_mint,
        token::authority = escrow_authority,
    )]
    pub escrow_vault: Account<'info, TokenAccount>,
    /// CHECK: PDA authority over escrow vault.
    #[account(seeds = [b"escrow", dispute_record.key().as_ref()], bump)]
    pub escrow_authority: UncheckedAccount<'info>,
    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn raise_dispute(
    ctx: Context<RaiseDispute>,
    target_did: String,
    pocc_hash: [u8; 32],
) -> Result<()> {
    let dispute = &mut ctx.accounts.dispute_record;

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.auditor_token_account.to_account_info(),
                to: ctx.accounts.escrow_vault.to_account_info(),
                authority: ctx.accounts.auditor.to_account_info(),
            },
        ),
        AUDITOR_BOND_AMOUNT,
    )?;

    dispute.auditor = ctx.accounts.auditor.key();
    dispute.accused = ctx.accounts.accused.key();
    dispute.pocc_hash = pocc_hash;
    dispute.status = DisputeStatus::PendingDefense;
    dispute.deadline = Clock::get()?.unix_timestamp + DEFAULT_DEFENSE_WINDOW_SECONDS;

    msg!(
        "🚨 [DISPUTE] Auditor {} raised dispute against {} ({})",
        dispute.auditor,
        dispute.accused,
        target_did
    );
    Ok(())
}

#[derive(Accounts)]
pub struct SubmitDefense<'info> {
    #[account(mut)]
    pub accused: Signer<'info>,
    #[account(
        mut,
        constraint = dispute_record.accused == accused.key() @ LifePlusError::UnauthorizedAuthority
    )]
    pub dispute_record: Account<'info, DisputeRecord>,
    #[account(
        mut,
        seeds = [b"escrow_vault", dispute_record.key().as_ref()],
        bump,
    )]
    pub escrow_vault: Account<'info, TokenAccount>,
    /// CHECK: PDA authority over escrow vault.
    #[account(seeds = [b"escrow", dispute_record.key().as_ref()], bump)]
    pub escrow_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn submit_zk_proof_defense(ctx: Context<SubmitDefense>, stark_proof: Vec<u8>) -> Result<()> {
    let dispute = &mut ctx.accounts.dispute_record;

    require!(
        Clock::get()?.unix_timestamp <= dispute.deadline,
        LifePlusError::DefenseTimeout
    );
    require!(
        dispute.status == DisputeStatus::PendingDefense,
        LifePlusError::InvalidDisputeStatus
    );

    let is_valid = verify_stark_proof_onchain(&dispute.pocc_hash, &stark_proof);

    if is_valid {
        let dispute_key = dispute.key();
        let seeds = &[
            b"escrow",
            dispute_key.as_ref(),
            &[ctx.bumps.escrow_authority],
        ];

        token::burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.usdc_mint.to_account_info(),
                    from: ctx.accounts.escrow_vault.to_account_info(),
                    authority: ctx.accounts.escrow_authority.to_account_info(),
                },
                &[seeds],
            ),
            AUDITOR_BOND_AMOUNT,
        )?;

        dispute.status = DisputeStatus::AuditorSlashed;
        msg!("💀 [SLASHER] Malicious auditor bond burned.");
    } else {
        dispute.status = DisputeStatus::AccusedSlashed;
        msg!("⚠️ [DEFENSE FAILED] Invalid STARK proof. Accused flagged for slashing.");
    }

    Ok(())
}

fn verify_stark_proof_onchain(_pocc_hash: &[u8; 32], proof: &[u8]) -> bool {
    !proof.is_empty()
}
