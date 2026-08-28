use anchor_lang::prelude::*;
use solana_program::sysvar::instructions::ID as IX_SYSVAR_ID;
use crate::creator_hash;
use crate::error::ClawtubeError;
use crate::event::Claimed;
use crate::state::{CreatorEscrow, Entry, EntryStatus, PlatformConfig, CLAIM_FEE_BPS, MAX_HANDLE, MAX_PLATFORM_UID};
pub fn claim<'info>(
    ctx: Context<'info, Claim<'info>>,
    platform_uid: String,
    handle: String,
    claim_nonce: u64,
    expires_at: i64,
    entry_indices: Vec<u64>,
) -> Result<()> {
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp < expires_at,
        ClawtubeError::AttestationExpired
    );

    let escrow = &mut ctx.accounts.escrow;
    require!(
        claim_nonce > escrow.last_claim_nonce,
        ClawtubeError::NonceReplay
    );
    require!(
        escrow.platform_uid == platform_uid,
        ClawtubeError::UidMismatch
    );
    require!(
        platform_uid.len() <= MAX_PLATFORM_UID,
        ClawtubeError::UidMismatch
    );
    require!(handle.len() <= MAX_HANDLE, ClawtubeError::UidMismatch);

    let oracle = ctx
        .accounts
        .config
        .oracle
        .ok_or(ClawtubeError::InvalidOracleSignature)?;

    // Construct expected attestation message
    let mut expected_msg = Vec::with_capacity(1 + 1 + 4 + platform_uid.len() + 32 + 8 + 8 + 4 + handle.len());
    expected_msg.push(1u8); // version 1
    expected_msg.push(escrow.platform);
    expected_msg.extend_from_slice(&(platform_uid.len() as u32).to_le_bytes());
    expected_msg.extend_from_slice(platform_uid.as_bytes());
    expected_msg.extend_from_slice(ctx.accounts.payout.key().as_ref());
    expected_msg.extend_from_slice(&claim_nonce.to_le_bytes());
    expected_msg.extend_from_slice(&expires_at.to_le_bytes());
    expected_msg.extend_from_slice(&(handle.len() as u32).to_le_bytes());
    expected_msg.extend_from_slice(handle.as_bytes());

    verify_oracle_attestation(
        &ctx.accounts.sysvar_instructions,
        &oracle,
        &expected_msg,
    )?;

    // Audit M2: fill handle on first claim if empty
    if escrow.handle.is_empty() {
        escrow.handle = handle;
    }

    require!(
        ctx.remaining_accounts.len() >= entry_indices.len(),
        ClawtubeError::InvalidEntryStatus
    );

    for (i, &index) in entry_indices.iter().enumerate() {
        let entry_account_info = &ctx.remaining_accounts[i];
        let (expected_entry_pda, _) = Pubkey::find_program_address(
            &[b"entry", escrow.creator_hash.as_ref(), &index.to_le_bytes()[..]],
            ctx.program_id,
        );
        require_keys_eq!(
            entry_account_info.key(),
            expected_entry_pda,
            ClawtubeError::InvalidEntryStatus
        );

        let mut entry_data = entry_account_info.try_borrow_mut_data()?;
        let mut entry = Entry::try_deserialize(&mut &entry_data[..])?;
        require_keys_eq!(
            *entry_account_info.owner,
            crate::ID,
            ClawtubeError::InvalidEntryStatus
        );
        require!(
            entry.status == EntryStatus::Active,
            ClawtubeError::InvalidEntryStatus
        );
        require!(
            clock.unix_timestamp < entry.expires_at,
            ClawtubeError::AttestationExpired
        );

        let amount = entry.amount;
        let fee = amount
            .checked_mul(CLAIM_FEE_BPS)
            .ok_or(ClawtubeError::VaultInvariantViolated)?
            .checked_div(10_000)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;
        let net = amount
            .checked_sub(fee)
            .ok_or(ClawtubeError::VaultInvariantViolated)?;

        if entry.mint.is_none() {
            let escrow_info = escrow.to_account_info();
            let treasury_info = ctx.accounts.treasury.to_account_info();
            let payout_info = ctx.accounts.payout.to_account_info();

            **escrow_info.try_borrow_mut_lamports()? = escrow_info
                .lamports()
                .checked_sub(amount)
                .ok_or(ClawtubeError::VaultInvariantViolated)?;
            **payout_info.try_borrow_mut_lamports()? = payout_info
                .lamports()
                .checked_add(net)
                .ok_or(ClawtubeError::VaultInvariantViolated)?;
            if fee > 0 {
                **treasury_info.try_borrow_mut_lamports()? = treasury_info
                    .lamports()
                    .checked_add(fee)
                    .ok_or(ClawtubeError::VaultInvariantViolated)?;
            }
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

        entry.status = EntryStatus::Claimed;
        entry.try_serialize(&mut &mut entry_data[..])?;

        emit!(Claimed {
            creator_hash: escrow.creator_hash,
            index,
            amount,
            mint: entry.mint,
            payout: ctx.accounts.payout.key(),
            claim_nonce,
        });
    }

    escrow.last_claim_nonce = claim_nonce;
    Ok(())
}

fn verify_oracle_attestation(
    sysvar_instructions: &AccountInfo,
    oracle: &Pubkey,
    expected_msg: &[u8],
) -> Result<()> {
    require_keys_eq!(
        *sysvar_instructions.key,
        IX_SYSVAR_ID,
        ClawtubeError::InvalidOracleSignature
    );

    let data = sysvar_instructions.try_borrow_data()?;
    require!(!data.is_empty(), ClawtubeError::InvalidOracleSignature);

    let oracle_bytes = oracle.as_ref();

    let mut found_oracle = false;
    let mut found_msg = false;

    if data.windows(oracle_bytes.len()).any(|w| w == oracle_bytes) {
        found_oracle = true;
    }
    if data.windows(expected_msg.len()).any(|w| w == expected_msg) {
        found_msg = true;
    }

    require!(
        found_oracle && found_msg,
        ClawtubeError::InvalidOracleSignature
    );
    Ok(())
}
#[derive(Accounts)]
#[instruction(platform_uid: String)]
pub struct Claim<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, PlatformConfig>,
    #[account(mut, seeds = [b"treasury"], bump)]
    pub treasury: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"escrow", creator_hash(escrow.platform, platform_uid.as_bytes()).as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, CreatorEscrow>,
    #[account(mut)]
    pub payout: Signer<'info>,
    /// CHECK: sysvar instructions account for Ed25519 introspection
    #[account(address = IX_SYSVAR_ID)]
    pub sysvar_instructions: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
