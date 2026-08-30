use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use crate::error::ClawtubeError;
use crate::event::TipReceived;
use crate::state::{CreatorEscrow, Entry, EntryStatus, Platform, PlatformConfig, DONATION_FEE_BPS, MAX_PLATFORM_UID};

pub fn tip_sol(
    ctx: Context<TipSol>,
    platform: u8,
    platform_uid: String,
    amount: u64,
    creator_hash: [u8; 16],
) -> Result<()> {
    require!(amount > 0, ClawtubeError::ZeroAmount);
    let _p = Platform::from_u8(platform)?;
    require!(
        ctx.accounts.config.is_platform_enabled(platform),
        ClawtubeError::PlatformDisabled
    );
    require!(
        platform_uid.len() <= MAX_PLATFORM_UID,
        ClawtubeError::UidMismatch
    );
    require!(
        creator_hash == crate::creator_hash(platform, platform_uid.as_bytes()),
        ClawtubeError::UidMismatch
    );

    let escrow = &mut ctx.accounts.escrow;
    let expected_hash = creator_hash;

    if escrow.creator_hash == [0u8; 16] {
        escrow.creator_hash = expected_hash;
        escrow.platform = platform;
        escrow.platform_uid = platform_uid.clone();
        escrow.handle = String::new(); // Audit M2: empty at init
        escrow.vault_bump = 0;
        escrow.entry_count = 0;
        escrow.last_claim_nonce = 0;
        escrow.vault_totals = [0; 3];
        escrow.bump = ctx.bumps.escrow;
    } else {
        require!(
            escrow.platform == platform && escrow.platform_uid == platform_uid,
            ClawtubeError::UidMismatch
        );
    }

    let fee = amount
        .checked_mul(DONATION_FEE_BPS)
        .ok_or(ClawtubeError::ZeroAmount)?
        .checked_div(10_000)
        .ok_or(ClawtubeError::ZeroAmount)?;
    let net = amount.checked_sub(fee).ok_or(ClawtubeError::ZeroAmount)?;

    if fee > 0 {
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                Transfer {
                    from: ctx.accounts.donor.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            fee,
        )?;
    }

    transfer(
        CpiContext::new(
            ctx.accounts.system_program.key(),
            Transfer {
                from: ctx.accounts.donor.to_account_info(),
                to: escrow.to_account_info(),
            },
        ),
        net,
    )?;

    let clock = Clock::get()?;
    let entry = &mut ctx.accounts.entry;
    entry.creator_hash = expected_hash;
    entry.index = escrow.entry_count;
    entry.amount = net;
    entry.mint = None;
    entry.donor = ctx.accounts.donor.key();
    entry.status = EntryStatus::Active;
    entry.expires_at = clock
        .unix_timestamp
        .checked_add(90 * 86_400)
        .ok_or(ClawtubeError::ZeroAmount)?;
    entry.sweep_at = clock
        .unix_timestamp
        .checked_add(120 * 86_400)
        .ok_or(ClawtubeError::ZeroAmount)?;
    entry.bump = ctx.bumps.entry;

    let current_index = escrow.entry_count;
    escrow.entry_count = escrow
        .entry_count
        .checked_add(1)
        .ok_or(ClawtubeError::NonceReplay)?;

    emit!(TipReceived {
        platform,
        creator_hash: expected_hash,
        index: current_index,
        amount: net,
        mint: None,
        donor: ctx.accounts.donor.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(platform: u8, platform_uid: String, amount: u64, creator_hash: [u8; 16])]
pub struct TipSol<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, PlatformConfig>,
    #[account(mut, seeds = [b"treasury"], bump)]
    pub treasury: SystemAccount<'info>,
    // init_if_needed is safe here: Anchor 1.x rejects reinitialization of an
    // already-initialized typed account (discriminator check), the PDA is
    // deterministic, and the handler re-validates creator_hash/uid on reuse.
    #[account(
        init_if_needed,
        payer = donor,
        space = CreatorEscrow::SIZE,
        seeds = [b"escrow", creator_hash.as_ref()],
        bump
    )]
    pub escrow: Account<'info, CreatorEscrow>,
    #[account(
        init,
        payer = donor,
        space = Entry::SIZE,
        seeds = [b"entry", creator_hash.as_ref(), &escrow.entry_count.to_le_bytes()[..]],
        bump
    )]
    pub entry: Account<'info, Entry>,
    #[account(mut)]
    pub donor: Signer<'info>,
    pub system_program: Program<'info, System>,
}
