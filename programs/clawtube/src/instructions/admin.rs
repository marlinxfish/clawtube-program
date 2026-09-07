use anchor_lang::prelude::*;
use anchor_spl::token::{transfer as spl_transfer, Token, TokenAccount, Transfer as SplTransfer};
use crate::error::ClawtubeError;
use crate::event::FeesWithdrawn;
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

pub fn withdraw_fees(
    ctx: Context<WithdrawFees>,
    pool: u8,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, ClawtubeError::ZeroAmount);
    let seed: &[u8] = match pool {
        0 => b"treasury",
        1 => b"buyback",
        2 => b"ops",
        _ => return Err(ClawtubeError::InvalidPool.into()),
    };
    let (expected_pda, bump) = Pubkey::find_program_address(&[seed], ctx.program_id);
    require_keys_eq!(
        ctx.accounts.pool_account.key(),
        expected_pda,
        ClawtubeError::VaultMismatch
    );

    let pool_lamports = ctx.accounts.pool_account.lamports();
    let min_rent = Rent::get()?.minimum_balance(0);
    let remaining = pool_lamports
        .checked_sub(amount)
        .ok_or(ClawtubeError::VaultInvariantViolated)?;
    require!(
        remaining == 0 || remaining >= min_rent,
        ClawtubeError::VaultInvariantViolated
    );
    let signer_seeds: &[&[u8]] = &[seed, &[bump]];
    anchor_lang::system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.key(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.pool_account.to_account_info(),
                to: ctx.accounts.recipient.to_account_info(),
            },
            &[signer_seeds],
        ),
        amount,
    )?;

    emit!(FeesWithdrawn {
        pool,
        amount,
        mint: None,
        recipient: ctx.accounts.recipient.key(),
    });

    Ok(())
}

pub fn withdraw_fees_spl(
    ctx: Context<WithdrawFeesSpl>,
    pool: u8,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, ClawtubeError::ZeroAmount);
    let seed: &[u8] = match pool {
        0 => b"treasury",
        1 => b"buyback",
        2 => b"ops",
        _ => return Err(ClawtubeError::InvalidPool.into()),
    };
    let (expected_pda, bump) = Pubkey::find_program_address(&[seed], ctx.program_id);
    require_keys_eq!(
        ctx.accounts.pool_account.key(),
        expected_pda,
        ClawtubeError::VaultMismatch
    );
    require_keys_eq!(
        ctx.accounts.pool_token_account.owner,
        expected_pda,
        ClawtubeError::VaultMismatch
    );

    let signer_seeds: &[&[u8]] = &[seed, &[bump]];
    spl_transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            SplTransfer {
                from: ctx.accounts.pool_token_account.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.pool_account.to_account_info(),
            },
            &[signer_seeds],
        ),
        amount,
    )?;

    emit!(FeesWithdrawn {
        pool,
        amount,
        mint: Some(ctx.accounts.pool_token_account.mint),
        recipient: ctx.accounts.recipient_token_account.key(),
    });

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

#[derive(Accounts)]
#[instruction(pool: u8)]
pub struct WithdrawFees<'info> {
    #[account(
        seeds = [b"config"],
        bump = config.bump,
        has_one = authority,
    )]
    pub config: Account<'info, PlatformConfig>,
    pub authority: Signer<'info>,
    /// CHECK: pool PDA (treasury, buyback, or ops) verified by seeds in handler
    #[account(mut)]
    pub pool_account: UncheckedAccount<'info>,
    /// CHECK: destination for SOL
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(pool: u8)]
pub struct WithdrawFeesSpl<'info> {
    #[account(
        seeds = [b"config"],
        bump = config.bump,
        has_one = authority,
    )]
    pub config: Account<'info, PlatformConfig>,
    pub authority: Signer<'info>,
    /// CHECK: pool PDA (treasury, buyback, or ops) verified by seeds in handler
    pub pool_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub pool_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
