use anchor_lang::prelude::*;
use crate::error::ClawtubeError;
use crate::event::Refunded;
use crate::state::{CreatorEscrow, Entry, EntryStatus, PlatformConfig};

pub fn refund<'info>(ctx: Context<'info, Refund<'info>>) -> Result<()> {
    let clock = Clock::get()?;
    let entry = &mut ctx.accounts.entry;
    let escrow = &mut ctx.accounts.escrow;

    require!(
        entry.status == EntryStatus::Active,
        ClawtubeError::InvalidEntryStatus
    );
    require!(
        clock.unix_timestamp >= entry.expires_at,
        ClawtubeError::EntryStillClaimable
    );
    require!(
        clock.unix_timestamp < entry.sweep_at,
        ClawtubeError::RefundWindowClosed
    );

    let amount = entry.amount;

    if entry.mint.is_none() {
        let escrow_info = escrow.to_account_info();
        let donor_info = ctx.accounts.donor.to_account_info();

        **escrow_info.try_borrow_mut_lamports()? = escrow_info
            .lamports()
            .checked_sub(amount)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
        **donor_info.try_borrow_mut_lamports()? = donor_info
            .lamports()
            .checked_add(amount)
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

    entry.status = EntryStatus::Refunded;

    emit!(Refunded {
        creator_hash: entry.creator_hash,
        index: entry.index,
        amount,
        mint: entry.mint,
        donor: ctx.accounts.donor.key(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct Refund<'info> {
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
        bump = entry.bump,
        constraint = entry.donor == donor.key() @ ClawtubeError::NotDonor
    )]
    pub entry: Account<'info, Entry>,
    #[account(mut)]
    pub donor: Signer<'info>,
    pub system_program: Program<'info, System>,
}
