use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{errors::LifePlusError, state::*};

const BIPS_DENOMINATOR: u128 = 10_000;

#[derive(Accounts)]
pub struct SettleWorkerCTx<'info> {
    #[account(mut)]
    pub orchestrator_escrow: Account<'info, TokenAccount>,
    #[account(mut)]
    pub worker_wallet: Account<'info, TokenAccount>,
    #[account(mut)]
    pub ctx_state: Account<'info, CompositeTaskState>,
    pub token_program: Program<'info, Token>,
    pub bank_authority: Signer<'info>,
}

pub fn process_x402_micro_settlement(
    ctx: Context<SettleWorkerCTx>,
    subtask_id: u64,
) -> Result<()> {
    let ctx_state = &mut ctx.accounts.ctx_state;

    require_keys_eq!(
        ctx_state.orchestrator,
        ctx.accounts.bank_authority.key(),
        LifePlusError::UnauthorizedAuthority
    );
    require!(
        ctx.accounts.orchestrator_escrow.mint == ctx.accounts.worker_wallet.mint,
        LifePlusError::MintMismatch
    );

    require!(
        ctx_state.completed_subtasks.contains(&subtask_id),
        LifePlusError::SubtaskNotVerified
    );
    require!(
        !ctx_state.settled_subtasks.contains(&subtask_id),
        LifePlusError::DoubleSpendingAttempt
    );

    let agreed_bips = ctx_state
        .subtask_rewards
        .iter()
        .find(|reward| reward.subtask_id == subtask_id)
        .ok_or(LifePlusError::SubtaskNotFound)?
        .payment_bips;

    let payout_amount_u128 = (ctx_state.total_bounty as u128)
        .checked_mul(agreed_bips as u128)
        .ok_or(LifePlusError::ArithmeticOverflow)?
        .checked_div(BIPS_DENOMINATOR)
        .ok_or(LifePlusError::ArithmeticOverflow)?;
    let payout_amount = u64::try_from(payout_amount_u128)
        .map_err(|_| error!(LifePlusError::ArithmeticOverflow))?;

    let cpi_accounts = Transfer {
        from: ctx.accounts.orchestrator_escrow.to_account_info(),
        to: ctx.accounts.worker_wallet.to_account_info(),
        authority: ctx.accounts.bank_authority.to_account_info(),
    };

    token::transfer(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
        payout_amount,
    )?;

    ctx_state.settled_subtasks.push(subtask_id);

    msg!(
        "💸 [x402 Cleared] Worker agent securely compensated {} LIFE++ for Subtask {}.",
        payout_amount,
        subtask_id
    );
    msg!(
        "🔒 [Security] BIPS payload derived from immutable chain state: {} / 10000",
        agreed_bips
    );

    Ok(())
}
