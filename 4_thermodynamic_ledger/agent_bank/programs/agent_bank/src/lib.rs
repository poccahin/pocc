use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("BankXxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");

pub const BIPS_DENOMINATOR: u128 = 10_000;
pub const MINIMUM_GENESIS_STAKE: u64 = 10_000_000;

#[program]
pub mod life_plus_agent_bank {
    use super::*;

    /// 边缘计算终端（挂机节点）提交 24 小时轧账数据。
    pub fn submit_daily_netting(
        ctx: Context<SubmitNetting>,
        merkle_root: [u8; 32],
        total_cleared_volume: u64,
        node_fee_bips: u16,
    ) -> Result<()> {
        require!(node_fee_bips <= 1_000, BankError::FeeRateTooHigh);

        let edge_node = &ctx.accounts.edge_node_record;

        // 1. 防女巫验证：节点必须持有最小 Genesis Stake。
        require!(
            edge_node.staked_life_plus >= MINIMUM_GENESIS_STAKE,
            BankError::InsufficientStake
        );

        // 2. 校验 AP2 路由诚实度。
        require!(!edge_node.is_slashed, BankError::NodeIsSlashed);

        // 3. 计算清算手续费：Fee = Volume * Rate / 10000。
        let clearing_fee_u128 = (total_cleared_volume as u128)
            .checked_mul(node_fee_bips as u128)
            .ok_or(BankError::ArithmeticOverflow)?
            .checked_div(BIPS_DENOMINATOR)
            .ok_or(BankError::ArithmeticOverflow)?;

        let clearing_fee =
            u64::try_from(clearing_fee_u128).map_err(|_| BankError::ArithmeticOverflow)?;

        // 4. 从总清算池划转手续费至节点钱包。
        let signer_seeds: &[&[&[u8]]] = &[&[b"bank-authority", &[ctx.bumps.bank_authority]]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.clearing_liquidity_pool.to_account_info(),
            to: ctx.accounts.edge_node_wallet.to_account_info(),
            authority: ctx.accounts.bank_authority.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, clearing_fee)?;

        // 5. 锚定默克尔根，广播全网结算状态。
        emit!(NettingSettledEvent {
            node_pubkey: ctx.accounts.edge_node_wallet.key(),
            merkle_root,
            cleared_volume: total_cleared_volume,
            fee_earned: clearing_fee,
        });

        msg!(
            "🏦 [Agent Bank] Daily Netting successful. Edge Node earned {} LIFE++ in clearing fees.",
            clearing_fee
        );

        Ok(())
    }
}

#[derive(Accounts)]
pub struct SubmitNetting<'info> {
    pub edge_node_record: Account<'info, EdgeNodeRecord>,

    /// 节点接收手续费的钱包（SPL Token Account）。
    #[account(
        mut,
        constraint = edge_node_wallet.owner == edge_node_record.node_wallet @ BankError::UnauthorizedNodeWallet,
        constraint = edge_node_wallet.mint == clearing_liquidity_pool.mint @ BankError::MintMismatch,
    )]
    pub edge_node_wallet: Account<'info, TokenAccount>,

    /// 全网清算资金池。
    #[account(mut)]
    pub clearing_liquidity_pool: Account<'info, TokenAccount>,

    /// Agent Bank 的 PDA 授权账户。
    #[account(seeds = [b"bank-authority"], bump)]
    pub bank_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[account]
pub struct EdgeNodeRecord {
    pub node_wallet: Pubkey,
    pub staked_life_plus: u64,
    pub is_slashed: bool,
}

#[event]
pub struct NettingSettledEvent {
    pub node_pubkey: Pubkey,
    pub merkle_root: [u8; 32],
    pub cleared_volume: u64,
    pub fee_earned: u64,
}

#[error_code]
pub enum BankError {
    #[msg("Node lacks the minimum LIFE++ Genesis Stake to perform clearing.")]
    InsufficientStake,
    #[msg("Node has been slashed for malicious AP2 routing. Netting denied.")]
    NodeIsSlashed,
    #[msg("Node fee rate exceeds protocol cap.")]
    FeeRateTooHigh,
    #[msg("Arithmetic overflow while calculating clearing fee.")]
    ArithmeticOverflow,
    #[msg("Edge node wallet is not authorized by the node record.")]
    UnauthorizedNodeWallet,
    #[msg("Clearing pool mint does not match edge node wallet mint.")]
    MintMismatch,
}
