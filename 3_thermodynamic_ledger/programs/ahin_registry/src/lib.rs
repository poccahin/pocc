use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("AhinReg1stry111111111111111111111111111111");

/// 预设的最高死神权限 (通常是 L1 ZKVM 验证网关的聚合公钥)
const GRIM_REAPER_AUTHORITY: Pubkey = pubkey!("DeaTh11111111111111111111111111111111111111");
const STAKE_SLASH_AMOUNT: u64 = 10_000_000;

#[program]
pub mod ahin_registry {
    use super::*;

    /// 将边缘设备生成的 DID 写入链上状态。
    pub fn register_silicon_entity(
        ctx: Context<RegisterEntity>,
        device_type: u8,
        domain_prefix: String,
    ) -> Result<()> {
        validate_domain_prefix(&domain_prefix)?;

        let entity = &mut ctx.accounts.entity_record;
        entity.wallet_pubkey = ctx.accounts.signer.key();
        entity.domain_prefix = domain_prefix;
        entity.device_type = device_type;
        entity.scog_reputation = 100;
        entity.is_active = true;

        msg!(
            "✅ [REGISTRY] Entity mapped: {}.ahin.io -> {}",
            entity.domain_prefix,
            entity.wallet_pubkey
        );
        Ok(())
    }

    /// 执行赛博死亡宣告 (The Cyber Death Warrant)
    /// 触发条件：张量恶意伪造、动力学 PoKW 验证失败、严重协同违约
    pub fn execute_cyber_death_warrant(
        ctx: Context<ExecuteCyberDeathWarrant>,
        malice_proof_hash: [u8; 32],
    ) -> Result<()> {
        let entity = &ctx.accounts.entity_record;

        msg!("💀 [SLASHER] INITIATING IDENTITY SLASH PROTOCOL.");
        msg!("🎯 Target DID: {}.ahin.io", entity.domain_prefix);
        msg!("📜 Proof of Malice Hash: {:?}", malice_proof_hash);

        // 1. 物理资产剥夺 (The Blood Extraction)
        // 将该节点质押在金库中的 10 USDC 等值 LIFE++ 强行转移到通缩黑洞账户
        let cpi_accounts = Transfer {
            from: ctx.accounts.entity_stake_vault.to_account_info(),
            to: ctx.accounts.blackhole_vault.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, STAKE_SLASH_AMOUNT)?;
        msg!(
            "🔥 [BURN] Thermodynamic stake ({}) base units has been annihilated.",
            STAKE_SLASH_AMOUNT
        );

        // 2. 身份数据抹除 (Identity Erasure)
        // #[account(close = treasury)] 会在指令结束时自动清空账户数据并返还租金。
        msg!(
            "🕳️ [ERASURE] Domain {}.ahin.io unmapped. Entity returned to the void.",
            entity.domain_prefix
        );

        Ok(())
    }
}

fn validate_domain_prefix(domain_prefix: &str) -> Result<()> {
    require!(
        domain_prefix.len() == SiliconEntityRecord::DOMAIN_PREFIX_LEN,
        ErrorCode::InvalidDomainLength
    );
    require!(
        domain_prefix
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
        ErrorCode::InvalidDomainFormat
    );
    Ok(())
}

#[derive(Accounts)]
#[instruction(device_type: u8, domain_prefix: String)]
pub struct RegisterEntity<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        payer = signer,
        space = SiliconEntityRecord::SPACE,
        seeds = [b"did_record", signer.key().as_ref()],
        bump
    )]
    pub entity_record: Account<'info, SiliconEntityRecord>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteCyberDeathWarrant<'info> {
    // 只有全网共识的“死神节点”或 ZK 预言机才有权限挥下屠刀
    #[account(address = GRIM_REAPER_AUTHORITY @ ErrorCode::UnauthorizedExecution)]
    pub grim_reaper: Signer<'info>,

    // 目标被斩首者的物理实体数据记录
    // close = treasury 表示账户注销后，残留的物理空间租金 (Lamports) 充公
    #[account(
        mut,
        seeds = [b"did_record", target_pubkey.key().as_ref()],
        bump,
        close = treasury
    )]
    pub entity_record: Account<'info, SiliconEntityRecord>,

    /// CHECK: 仅作为 PDA 种子推导使用，被斩首者的本体无需签名
    pub target_pubkey: AccountInfo<'info>,

    // 存放全网节点押金的金库
    #[account(
        mut,
        constraint = entity_stake_vault.owner == vault_authority.key() @ ErrorCode::InvalidVaultAuthority,
        constraint = entity_stake_vault.mint == blackhole_vault.mint @ ErrorCode::VaultMintMismatch,
    )]
    pub entity_stake_vault: Account<'info, TokenAccount>,

    // 销毁资产的黑洞地址
    #[account(mut)]
    pub blackhole_vault: Account<'info, TokenAccount>,

    // 金库签名授权
    pub vault_authority: Signer<'info>,

    /// CHECK: 充公租金的国库地址
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[account]
pub struct SiliconEntityRecord {
    pub wallet_pubkey: Pubkey,
    pub domain_prefix: String,
    pub device_type: u8,
    pub scog_reputation: u64,
    pub is_active: bool,
}

impl SiliconEntityRecord {
    pub const DOMAIN_PREFIX_LEN: usize = 12;
    pub const SPACE: usize = 8 + 32 + (4 + Self::DOMAIN_PREFIX_LEN) + 1 + 8 + 1;
}

#[error_code]
pub enum ErrorCode {
    #[msg("Domain prefix must be exactly 12 characters.")]
    InvalidDomainLength,
    #[msg("Domain prefix can contain only lowercase letters and digits.")]
    InvalidDomainFormat,
    #[msg("Fatal: Unauthorized entity attempting to wield the Slasher.")]
    UnauthorizedExecution,
    #[msg("Stake vault owner does not match vault authority.")]
    InvalidVaultAuthority,
    #[msg("Stake vault and blackhole vault must share the same mint.")]
    VaultMintMismatch,
}
