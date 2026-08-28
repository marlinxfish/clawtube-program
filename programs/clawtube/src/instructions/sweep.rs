use anchor_lang::prelude::*;
use crate::error::ClawtubeError;
use crate::event::Swept;
use crate::state::{CreatorEscrow, Entry, EntryStatus, PlatformConfig};

pub fn sweep<'info>(ctx: Context<'info, Sweep<'info>>) -> Result<()> {
    let clock = Clock::get()?;
    let entry = &mut ctx.accounts.entry;
    let escrow = &mut ctx.accounts.escrow;

    require!(
        entry.status == EntryStatus::Active,
        ClawtubeError::InvalidEntryStatus
    );
    require!(
        clock.unix_timestamp >= entry.sweep_at,
        ClawtubeError::SweepWindowNotOpen
    );

    let amount = entry.amount;
    let b70 = amount
        .checked_mul(70)
        .ok_or(ClawtubeError::VaultInvariantViolated)?
        .checked_div(100)
        .ok_or(ClawtubeError::VaultInvariantViolated)?;
    let o20 = amount
        .checked_mul(20)
        .ok_or(ClawtubeError::VaultInvariantViolated)?
        .checked_div(100)
        .ok_or(ClawtubeError::VaultInvariantViolated)?;
    let s10 = amount
        .checked_sub(b70)
        .ok_or(ClawtubeError::VaultInvariantViolated)?
        .checked_sub(o20)
        .ok_or(ClawtubeError::VaultInvariantViolated)?;

    if entry.mint.is_none() {
        let escrow_info = escrow.to_account_info();
        let buyback_info = ctx.accounts.buyback.to_account_info();
        let ops_info = ctx.accounts.ops.to_account_info();
        let sweeper_info = ctx.accounts.sweeper.to_account_info();

        **escrow_info.try_borrow_mut_lamports()? = escrow_info
            .lamports()
            .checked_sub(amount)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
        **buyback_info.try_borrow_mut_lamports()? = buyback_info
            .lamports()
            .checked_add(b70)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
        **ops_info.try_borrow_mut_lamports()? = ops_info
            .lamports()
            .checked_add(o20)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
        **sweeper_info.try_borrow_mut_lamports()? = sweeper_info
            .lamports()
            .checked_add(s10)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
    } else {
        let mint_key = entry.mint.unwrap();
        let slot = ctx
            .accounts
            .config
            .is_mint_whitelisted(&mint_key)
            .ok_or(ClawtubeError::MintNotWhitelisted)?;
        escrow.vault_totals[slot] = escrow.vault_totals[slot]
            .checked_sub(amount)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
    }

    entry.status = EntryStatus::Swept;

    emit!(Swept {
        creator_hash: entry.creator_hash,
        index: entry.index,
        amount,
        mint: entry.mint,
        sweeper: ctx.accounts.sweeper.key(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct Sweep<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, PlatformConfig>,
    #[account(
        mut,
        seeds = [b"escrow", entry.creator_hash.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, CreatorEscrow>,
    #[account(
        mut,
        seeds = [b"entry", entry.creator_hash.as_ref(), &entry.index.to_le_bytes()[..]],
        bump = entry.bump
    )]
    pub entry: Account<'info, Entry>,
    #[account(mut, seeds = [b"buyback"], bump)]
    pub buyback: SystemAccount<'info>,
    #[account(mut, seeds = [b"ops"], bump)]
    pub ops: SystemAccount<'info>,
    #[account(mut)]
    pub sweeper: Signer<'info>,
    pub system_program: Program<'info, System>,
}
