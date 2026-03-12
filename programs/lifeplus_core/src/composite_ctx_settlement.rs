use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::{errors::LifePlusError, state::*};

const BIPS_DENOMINATOR: u128 = 10_000;
const BURN_TAX_DENOMINATOR: u64 = 100;
const MAX_BATCH_SIZE: u32 = 200;

#[derive(Accounts)]
#[instruction(task_id: [u8; 32])]
pub struct SettleWorkerCTx<'info> {
    #[account(mut)]
    pub orchestrator_escrow: Account<'info, TokenAccount>,
    #[account(mut)]
    pub worker_wallet: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"composite_task", bank_authority.key().as_ref(), task_id.as_ref()],
        bump,
    )]
    pub ctx_state: Account<'info, CompositeTaskState>,
    pub token_program: Program<'info, Token>,
    pub bank_authority: Signer<'info>,
}

pub fn process_x402_micro_settlement(ctx: Context<SettleWorkerCTx>, task_id: [u8; 32], subtask_id: u64) -> Result<()> {
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
        ctx.accounts.orchestrator_escrow.mint == ctx.accounts.token_mint.key(),
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

    let agreed_reward = ctx_state
        .subtask_rewards
        .iter()
        .find(|reward| reward.subtask_id == subtask_id)
        .ok_or(LifePlusError::SubtaskNotFound)?;

    let agreed_bips = agreed_reward.payment_bips;

    require_keys_eq!(
        ctx.accounts.worker_wallet.owner,
        agreed_reward.worker,
        LifePlusError::WorkerWalletMismatch
    );

    let payout_amount_u128 = (ctx_state.total_bounty as u128)
        .checked_mul(agreed_bips as u128)
        .ok_or(LifePlusError::ArithmeticOverflow)?
        .checked_div(BIPS_DENOMINATOR)
        .ok_or(LifePlusError::ArithmeticOverflow)?;
    let total_payout =
        u64::try_from(payout_amount_u128).map_err(|_| error!(LifePlusError::ArithmeticOverflow))?;

    let burn_tax = total_payout
        .checked_div(BURN_TAX_DENOMINATOR)
        .ok_or(LifePlusError::ArithmeticOverflow)?;
    let net_payout = total_payout
        .checked_sub(burn_tax)
        .ok_or(LifePlusError::ArithmeticOverflow)?;

    let cpi_accounts = Transfer {
        from: ctx.accounts.orchestrator_escrow.to_account_info(),
        to: ctx.accounts.worker_wallet.to_account_info(),
        authority: ctx.accounts.bank_authority.to_account_info(),
    };

    token::transfer(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
        net_payout,
    )?;

    let burn_accounts = Burn {
        mint: ctx.accounts.token_mint.to_account_info(),
        from: ctx.accounts.orchestrator_escrow.to_account_info(),
        authority: ctx.accounts.bank_authority.to_account_info(),
    };

    token::burn(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), burn_accounts),
        burn_tax,
    )?;

    ctx_state.settled_subtasks.push(subtask_id);

    msg!(
        "💸 [x402 Cleared] Worker agent securely compensated {} LIFE++ for Subtask {}.",
        net_payout,
        subtask_id
    );
    msg!(
        "🔥 [Deflation] Cognitive friction burn executed: {} LIFE++ permanently destroyed.",
        burn_tax
    );
    msg!(
        "🔒 [Security] BIPS payload derived from immutable chain state: {} / 10000",
        agreed_bips
    );

    Ok(())
}

pub fn process_batch_settlement_safe(ctx: Context<SettleWorkerCTx>, task_id: [u8; 32], batch_size: u32) -> Result<()> {
    let state = &mut ctx.accounts.ctx_state;

    require_keys_eq!(
        state.orchestrator,
        ctx.accounts.bank_authority.key(),
        LifePlusError::UnauthorizedAuthority
    );
    require!(
        ctx.accounts.orchestrator_escrow.mint == ctx.accounts.worker_wallet.mint,
        LifePlusError::MintMismatch
    );
    require!(
        ctx.accounts.orchestrator_escrow.mint == ctx.accounts.token_mint.key(),
        LifePlusError::MintMismatch
    );
    require!(!state.is_fully_settled, LifePlusError::AlreadySettled);
    require!(batch_size > 0, LifePlusError::InvalidBatchSize);
    require!(
        batch_size <= MAX_BATCH_SIZE,
        LifePlusError::BatchSizeTooLarge
    );

    let start_idx = state.last_processed_index as usize;
    let total_len = state.subtask_rewards.len();
    require!(start_idx < total_len, LifePlusError::AlreadySettled);

    let end_idx = core::cmp::min(start_idx + batch_size as usize, total_len);

    for i in start_idx..end_idx {
        let subtask_id = state.subtask_rewards[i].subtask_id;
        let payment_bips = state.subtask_rewards[i].payment_bips;
        let subtask_worker = state.subtask_rewards[i].worker;

        require!(
            state.completed_subtasks.contains(&subtask_id),
            LifePlusError::SubtaskNotVerified
        );
        require!(
            !state.settled_subtasks.contains(&subtask_id),
            LifePlusError::DoubleSpendingAttempt
        );

        require_keys_eq!(
            ctx.accounts.worker_wallet.owner,
            subtask_worker,
            LifePlusError::WorkerWalletMismatch
        );
        // NOTE: batch settlement processes subtasks belonging to the same worker only.
        // If a subtask in the batch belongs to a different worker, WorkerWalletMismatch
        // is returned and the entire batch is rejected.

        let payout_amount_u128 = (state.total_bounty as u128)
            .checked_mul(payment_bips as u128)
            .ok_or(LifePlusError::ArithmeticOverflow)?
            .checked_div(BIPS_DENOMINATOR)
            .ok_or(LifePlusError::ArithmeticOverflow)?;
        let total_payout = u64::try_from(payout_amount_u128)
            .map_err(|_| error!(LifePlusError::ArithmeticOverflow))?;

        let burn_tax = total_payout
            .checked_div(BURN_TAX_DENOMINATOR)
            .ok_or(LifePlusError::ArithmeticOverflow)?;
        let net_payout = total_payout
            .checked_sub(burn_tax)
            .ok_or(LifePlusError::ArithmeticOverflow)?;

        let transfer_accounts = Transfer {
            from: ctx.accounts.orchestrator_escrow.to_account_info(),
            to: ctx.accounts.worker_wallet.to_account_info(),
            authority: ctx.accounts.bank_authority.to_account_info(),
        };

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
            ),
            net_payout,
        )?;

        let burn_accounts = Burn {
            mint: ctx.accounts.token_mint.to_account_info(),
            from: ctx.accounts.orchestrator_escrow.to_account_info(),
            authority: ctx.accounts.bank_authority.to_account_info(),
        };

        token::burn(
            CpiContext::new(ctx.accounts.token_program.to_account_info(), burn_accounts),
            burn_tax,
        )?;

        state.settled_subtasks.push(subtask_id);
    }

    state.last_processed_index = end_idx as u32;
    if state.last_processed_index as usize == total_len {
        state.is_fully_settled = true;
    }

    Ok(())
}
