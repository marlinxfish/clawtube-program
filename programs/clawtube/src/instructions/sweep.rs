use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::error::ClawtubeError;
use crate::event::Swept;
use crate::instructions::util::{require_token_account, transfer_spl_from_vault};
use crate::state::{CreatorEscrow, Entry, EntryStatus, PlatformConfig};

pub fn sweep<'info>(ctx: Context<'info, Sweep<'info>>) -> Result<()> {
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
        clock.unix_timestamp >= ctx.accounts.entry.sweep_at,
        ClawtubeError::SweepWindowNotOpen
    );

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

    if mint.is_none() {
        require!(ctx.remaining_accounts.is_empty(), ClawtubeError::VaultMismatch);
        let escrow_info = ctx.accounts.escrow.to_account_info();
        let buyback_info = ctx.accounts.buyback.to_account_info();
        let ops_info = ctx.accounts.ops.to_account_info();
        let sweeper_info = ctx.accounts.sweeper.to_account_info();

        let escrow_min = Rent::get()?.minimum_balance(escrow_info.data_len());
        let new_lamports = escrow_info
            .lamports()
            .checked_sub(amount)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
        require!(new_lamports >= escrow_min, ClawtubeError::VaultInvariantViolated);

        **escrow_info.try_borrow_mut_lamports()? = new_lamports;
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
        let mint_key = mint.unwrap();
        let slot = ctx
            .accounts
            .config
            .is_mint_whitelisted(&mint_key)
            .ok_or(ClawtubeError::MintNotWhitelisted)?;

        require!(
            ctx.remaining_accounts.len() >= 4,
            ClawtubeError::VaultMismatch
        );
        let vault = &ctx.remaining_accounts[0];
        let buyback_ata = &ctx.remaining_accounts[1];
        let ops_ata = &ctx.remaining_accounts[2];
        let sweeper_ata = &ctx.remaining_accounts[3];

        require_token_account(
            vault,
            &ctx.accounts.token_program.key(),
            &mint_key,
            &ctx.accounts.escrow.key(),
        )?;
        require_token_account(
            buyback_ata,
            &ctx.accounts.token_program.key(),
            &mint_key,
            &ctx.accounts.buyback.key(),
        )?;
        require_token_account(
            ops_ata,
            &ctx.accounts.token_program.key(),
            &mint_key,
            &ctx.accounts.ops.key(),
        )?;
        require_token_account(
            sweeper_ata,
            &ctx.accounts.token_program.key(),
            &mint_key,
            &ctx.accounts.sweeper.key(),
        )?;

        let escrow_info = ctx.accounts.escrow.to_account_info();
        let escrow_hash = ctx.accounts.escrow.creator_hash;
        let escrow_bump = ctx.accounts.escrow.bump;

        transfer_spl_from_vault(&ctx.accounts.token_program, vault, buyback_ata, &escrow_info, escrow_hash, escrow_bump, b70)?;
        transfer_spl_from_vault(&ctx.accounts.token_program, vault, ops_ata, &escrow_info, escrow_hash, escrow_bump, o20)?;
        transfer_spl_from_vault(&ctx.accounts.token_program, vault, sweeper_ata, &escrow_info, escrow_hash, escrow_bump, s10)?;

        ctx.accounts.escrow.vault_totals[slot] = ctx.accounts.escrow.vault_totals[slot]
            .checked_sub(amount)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
    }

    ctx.accounts.entry.status = EntryStatus::Swept;

    emit!(Swept {
        creator_hash: entry_hash,
        index: entry_index,
        amount,
        mint,
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
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
