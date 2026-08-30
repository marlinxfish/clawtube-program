use anchor_lang::prelude::*;
use crate::error::ClawtubeError;
use crate::state::PlatformConfig;

pub fn initialize_config(
    ctx: Context<InitializeConfig>,
    platform_registry: u8,
    oracle: Option<Pubkey>,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.authority = ctx.accounts.authority.key();
    config.oracle = oracle;
    config.platform_registry = platform_registry;
    config.spl_mints = [Pubkey::default(); 3];
    config.spl_mint_count = 0;
    config.bump = ctx.bumps.config;
    Ok(())
}

pub fn update_authority(ctx: Context<UpdateConfig>, new_authority: Pubkey) -> Result<()> {
    require!(
        new_authority != Pubkey::default(),
        ClawtubeError::ZeroPubkey
    );
    ctx.accounts.config.authority = new_authority;
    Ok(())
}

pub fn set_oracle(ctx: Context<UpdateConfig>, oracle: Option<Pubkey>) -> Result<()> {
    if let Some(o) = oracle {
        require!(o != Pubkey::default(), ClawtubeError::ZeroPubkey);
    }
    ctx.accounts.config.oracle = oracle;
    Ok(())
}

pub fn update_whitelist(ctx: Context<UpdateConfig>, mints: Vec<Pubkey>) -> Result<()> {
    require!(mints.len() <= 3, ClawtubeError::WhitelistOverflow);
    for (i, mint) in mints.iter().enumerate() {
        require!(*mint != Pubkey::default(), ClawtubeError::DuplicateMint);
        require!(
            !mints[..i].contains(mint),
            ClawtubeError::DuplicateMint
        );
    }
    let config = &mut ctx.accounts.config;
    config.spl_mint_count = mints.len() as u8;
    for (i, mint) in mints.iter().enumerate() {
        config.spl_mints[i] = *mint;
    }
    // Zero out remaining slots
    for i in mints.len()..3 {
        config.spl_mints[i] = Pubkey::default();
    }
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = PlatformConfig::SIZE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        has_one = authority,
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, PlatformConfig>,
    pub authority: Signer<'info>,
}
