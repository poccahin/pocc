use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

pub const MIN_ENTRY_VALUE_USDT: u64 = 10;
pub const LIFE_PLUS_MINT: Pubkey = pubkey!("7YdwpERJjzw7UVojxLpvu5ycKBRdYaxaKn4HvoHLpump");

pub fn execute_mandatory_buy_in(
    ctx: Context<RegisterAgentPersona>,
    required_life_plus_amount: u64,
) -> Result<()> {
    require!(
        required_life_plus_amount > 0,
        RegistrationGatewayError::InvalidRequiredStake
    );

    let cpi_accounts = Transfer {
        from: ctx.accounts.agent_token_account.to_account_info(),
        to: ctx.accounts.protocol_staking_pool.to_account_info(),
        authority: ctx.accounts.agent_owner.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    token::transfer(cpi_ctx, required_life_plus_amount)?;

    let persona = &mut ctx.accounts.agent_persona;
    persona.owner = ctx.accounts.agent_owner.key();
    persona.staked_amount = required_life_plus_amount;
    persona.is_active = true;
    persona.scog_score = 100;
    persona.bump = ctx.bumps.agent_persona;

    msg!("🚀 [GATEWAY] 10 USDT equivalent LIFE++ locked for activation.");
    msg!("✅ [GATEWAY] Agent Persona Activated. Welcome to the thermodynamic economy.");

    Ok(())
}

#[derive(Accounts)]
pub struct RegisterAgentPersona<'info> {
    #[account(mut)]
    pub agent_owner: Signer<'info>,

    #[account(
        mut,
        token::authority = agent_owner,
        constraint = agent_token_account.mint == LIFE_PLUS_MINT @ RegistrationGatewayError::InvalidLifePlusMint,
    )]
    pub agent_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = protocol_staking_pool.mint == LIFE_PLUS_MINT @ RegistrationGatewayError::InvalidLifePlusMint,
    )]
    pub protocol_staking_pool: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = agent_owner,
        space = 8 + AgentPersona::INIT_SPACE,
        seeds = [b"persona", agent_owner.key().as_ref()],
        bump
    )]
    pub agent_persona: Account<'info, AgentPersona>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct AgentPersona {
    pub owner: Pubkey,
    pub staked_amount: u64,
    pub is_active: bool,
    pub scog_score: u16,
    pub bump: u8,
}

#[error_code]
pub enum RegistrationGatewayError {
    #[msg("LIFE++ mint is invalid for mandatory buy-in.")]
    InvalidLifePlusMint,
    #[msg("Required LIFE++ stake must be greater than zero.")]
    InvalidRequiredStake,
    #[msg("Quoted entry value is below the 10 USDT minimum.")]
    EntryValueTooLow,
}
