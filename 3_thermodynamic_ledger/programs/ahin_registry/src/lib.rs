use anchor_lang::prelude::*;

declare_id!("AhinReg1stry111111111111111111111111111111");

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
}
