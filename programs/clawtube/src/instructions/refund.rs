use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::error::ClawtubeError;
use crate::event::Refunded;
use crate::instructions::util::{require_token_account, transfer_spl_from_vault};
use crate::state::{CreatorEscrow, Entry, EntryStatus, PlatformConfig};

pub fn refund<'info>(ctx: Context<'info, Refund<'info>>) -> Result<()> {
    let clock = Clock::get()?;

    let amount = ctx.accounts.entry.amount;
    let mint = ctx.accounts.entry.mint;
    let entry_hash = ctx.accounts.entry.creator_hash;
    let entry_index = ctx.accounts.entry.index;

    require!(
        ctx.accounts.entry.status == EntryStatus::Active,
        ClawtubeError::InvalidEntryStatus
    );
    require!(
        clock.unix_timestamp >= ctx.accounts.entry.expires_at,
        ClawtubeError::EntryStillClaimable
    );
    require!(
        clock.unix_timestamp < ctx.accounts.entry.sweep_at,
        ClawtubeError::RefundWindowClosed
    );

    if mint.is_none() {
        require!(ctx.remaining_accounts.is_empty(), ClawtubeError::VaultMismatch);
        let escrow_info = ctx.accounts.escrow.to_account_info();
        let donor_info = ctx.accounts.donor.to_account_info();

        let escrow_min = Rent::get()?.minimum_balance(escrow_info.data_len());
        let new_lamports = escrow_info
            .lamports()
            .checked_sub(amount)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
        require!(new_lamports >= escrow_min, ClawtubeError::VaultInvariantViolated);

        **escrow_info.try_borrow_mut_lamports()? = new_lamports;
        **donor_info.try_borrow_mut_lamports()? = donor_info
            .lamports()
            .checked_add(amount)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
    } else {
        let mint_key = mint.unwrap();
        let slot = ctx
            .accounts
            .config
            .is_mint_whitelisted(&mint_key)
            .ok_or(ClawtubeError::MintNotWhitelisted)?;

        require!(
            ctx.remaining_accounts.len() >= 2,
            ClawtubeError::VaultMismatch
        );
        let vault = &ctx.remaining_accounts[0];
        let donor_ata = &ctx.remaining_accounts[1];

        require_token_account(
            vault,
            &ctx.accounts.token_program.key(),
            &mint_key,
            &ctx.accounts.escrow.key(),
        )?;
        require_token_account(
            donor_ata,
            &ctx.accounts.token_program.key(),
            &mint_key,
            &ctx.accounts.donor.key(),
        )?;

        let escrow_info = ctx.accounts.escrow.to_account_info();
        transfer_spl_from_vault(
            &ctx.accounts.token_program,
            vault,
            donor_ata,
            &escrow_info,
            ctx.accounts.escrow.creator_hash,
            ctx.accounts.escrow.bump,
            amount,
        )?;

        ctx.accounts.escrow.vault_totals[slot] = ctx.accounts.escrow.vault_totals[slot]
            .checked_sub(amount)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
    }

    ctx.accounts.entry.status = EntryStatus::Refunded;

    emit!(Refunded {
        creator_hash: entry_hash,
        index: entry_index,
        amount,
        mint,
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
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
