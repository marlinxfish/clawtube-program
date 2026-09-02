use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use solana_program::ed25519_program::ID as ED25519_PROGRAM_ID;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked, ID as IX_SYSVAR_ID,
};
use crate::error::ClawtubeError;
use crate::event::Claimed;
use crate::instructions::util::{require_token_account, transfer_spl_from_vault};
use crate::state::{
    CreatorEscrow, Entry, EntryStatus, Platform, PlatformConfig, CLAIM_FEE_BPS, MAX_HANDLE,
    MAX_PLATFORM_UID,
};

pub fn claim<'info>(
    ctx: Context<'info, Claim<'info>>,
    platform: u8,
    platform_uid: String,
    handle: String,
    claim_nonce: u64,
    expires_at: i64,
    entry_indices: Vec<u64>,
    mint: Option<Pubkey>,
    creator_hash: [u8; 16],
) -> Result<()> {
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp < expires_at,
        ClawtubeError::AttestationExpired
    );
    require!(platform_uid.len() <= MAX_PLATFORM_UID, ClawtubeError::UidMismatch);
    require!(handle.len() <= MAX_HANDLE, ClawtubeError::UidMismatch);
    Platform::from_u8(platform)?;
    require!(
        creator_hash == crate::creator_hash(platform, platform_uid.as_bytes()),
        ClawtubeError::UidMismatch
    );

    {
        let escrow = &ctx.accounts.escrow;
        require!(claim_nonce > escrow.last_claim_nonce, ClawtubeError::NonceReplay);
        require!(escrow.platform == platform, ClawtubeError::PlatformMismatch);
        require!(escrow.platform_uid == platform_uid, ClawtubeError::UidMismatch);
    }

    let oracle = ctx
        .accounts
        .config
        .oracle
        .ok_or(ClawtubeError::InvalidOracleSignature)?;

    // Reconstruct the exact attestation message the oracle signs (v1).
    let mut expected_msg = Vec::with_capacity(
        1 + 1 + 4 + platform_uid.len() + 32 + 8 + 8 + 4 + handle.len(),
    );
    expected_msg.push(1u8); // version 1
    expected_msg.push(platform);
    expected_msg.extend_from_slice(&(platform_uid.len() as u32).to_le_bytes());
    expected_msg.extend_from_slice(platform_uid.as_bytes());
    expected_msg.extend_from_slice(ctx.accounts.payout.key().as_ref());
    expected_msg.extend_from_slice(&claim_nonce.to_le_bytes());
    expected_msg.extend_from_slice(&expires_at.to_le_bytes());
    expected_msg.extend_from_slice(&(handle.len() as u32).to_le_bytes());
    expected_msg.extend_from_slice(handle.as_bytes());

    verify_oracle_attestation(&ctx.accounts.sysvar_instructions.to_account_info(), &oracle, &expected_msg)?;

    // Update handle on claim if changed (e.g. social media username update).
    // The oracle-signed attestation binds handle to platform_uid and payout.
    {
        let escrow = &mut ctx.accounts.escrow;
        if escrow.handle != handle {
            escrow.handle = handle.clone();
        }
    }

    let ata_offset = entry_indices.len();
    require!(
        ctx.remaining_accounts.len() >= ata_offset,
        ClawtubeError::InvalidEntryStatus
    );
    if mint.is_some() {
        require!(
            ctx.remaining_accounts.len() >= ata_offset + 3,
            ClawtubeError::VaultMismatch
        );
    }

    let escrow_hash = ctx.accounts.escrow.creator_hash;
    let escrow_bump = ctx.accounts.escrow.bump;

    for (i, &index) in entry_indices.iter().enumerate() {
        let entry_account_info = &ctx.remaining_accounts[i];
        let (expected_entry_pda, _) = Pubkey::find_program_address(
            &[b"entry", escrow_hash.as_ref(), &index.to_le_bytes()[..]],
            ctx.program_id,
        );
        require_keys_eq!(
            entry_account_info.key(),
            expected_entry_pda,
            ClawtubeError::InvalidEntryStatus
        );
        require_keys_eq!(
            *entry_account_info.owner,
            crate::ID,
            ClawtubeError::InvalidEntryStatus
        );

        let mut entry_data = entry_account_info.try_borrow_mut_data()?;
        let mut entry = Entry::try_deserialize(&mut &entry_data[..])?;
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
            require!(mint.is_none(), ClawtubeError::VaultMismatch);
            // SOL payout via manual lamport transfer (escrow holds raw lamports).
            let escrow_info = ctx.accounts.escrow.to_account_info();
            let treasury_info = ctx.accounts.treasury.to_account_info();
            let payout_info = ctx.accounts.payout.to_account_info();

            let escrow_min = Rent::get()?.minimum_balance(escrow_info.data_len());
            let new_lamports = escrow_info
                .lamports()
                .checked_sub(amount)
                .ok_or(ClawtubeError::VaultInvariantViolated)?;
            require!(new_lamports >= escrow_min, ClawtubeError::VaultInvariantViolated);

            **escrow_info.try_borrow_mut_lamports()? = new_lamports;
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
            require!(mint == Some(mint_key), ClawtubeError::VaultMismatch);
            let slot = ctx
                .accounts
                .config
                .is_mint_whitelisted(&mint_key)
                .ok_or(ClawtubeError::MintNotWhitelisted)?;

            let vault = &ctx.remaining_accounts[ata_offset];
            let payout_ata = &ctx.remaining_accounts[ata_offset + 1];
            let treasury_ata = &ctx.remaining_accounts[ata_offset + 2];

            require_token_account(
                vault,
                &ctx.accounts.token_program.key(),
                &mint_key,
                &ctx.accounts.escrow.key(),
            )?;
            require_token_account(
                payout_ata,
                &ctx.accounts.token_program.key(),
                &mint_key,
                &ctx.accounts.payout.key(),
            )?;
            require_token_account(
                treasury_ata,
                &ctx.accounts.token_program.key(),
                &mint_key,
                &ctx.accounts.treasury.key(),
            )?;

            let escrow_info = ctx.accounts.escrow.to_account_info();
            transfer_spl_from_vault(
                &ctx.accounts.token_program,
                vault,
                payout_ata,
                &escrow_info,
                escrow_hash,
                escrow_bump,
                net,
            )?;
            if fee > 0 {
                transfer_spl_from_vault(
                    &ctx.accounts.token_program,
                    vault,
                    treasury_ata,
                    &escrow_info,
                    escrow_hash,
                    escrow_bump,
                    fee,
                )?;
            }

            ctx.accounts.escrow.vault_totals[slot] = ctx.accounts.escrow.vault_totals[slot]
                .checked_sub(amount)
                .ok_or(ClawtubeError::VaultInvariantViolated)?;
        }

        entry.status = EntryStatus::Claimed;
        entry.try_serialize(&mut &mut entry_data[..])?;
        drop(entry_data);

        emit!(Claimed {
            creator_hash: escrow_hash,
            index,
            amount,
            mint: entry.mint,
            payout: ctx.accounts.payout.key(),
            claim_nonce,
        });
    }

    ctx.accounts.escrow.last_claim_nonce = claim_nonce;
    Ok(())
}

/// Verify the oracle signed `expected_msg` by locating an Ed25519 signature
/// verification instruction (for the oracle pubkey, over the exact message)
/// among the transaction instructions that precede this one.
///
/// The `Ed25519SigVerify` native program rejects the whole transaction when a
/// signature is invalid, so finding a matching instruction proves the oracle
/// actually signed the message — a raw byte search would NOT.
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

    let current_index = load_current_index_checked(sysvar_instructions)?;
    for i in 0..current_index {
        let ix = load_instruction_at_checked(i as usize, sysvar_instructions)?;
        if ix.program_id != ED25519_PROGRAM_ID {
            continue;
        }
        if let Some((pk, msg)) = parse_ed25519_instruction(&ix.data) {
            if pk == *oracle && msg == expected_msg {
                return Ok(());
            }
        }
    }
    Err(ClawtubeError::InvalidOracleSignature.into())
}

/// Parse a single-signature Ed25519 verification instruction's data, returning
/// the verified public key and message.
///
/// Layout: num_signatures(u8) padding(u8) signatures_offset(u16)
/// signatures_ix(u16) public_key_offset(u16) public_key_ix(u16)
/// message_offset(u16) message_size(u16) message_ix(u16), then the packed
/// signature(64) + public_key(32) + message(message_size) bytes.
pub(crate) fn parse_ed25519_instruction(data: &[u8]) -> Option<(Pubkey, &[u8])> {
    if data.len() < 16 {
        return None;
    }
    if data[0] != 1 {
        return None; // we only ever sign a single message
    }
    let signature_offset = u16::from_le_bytes([data[2], data[3]]) as usize;
    let signature_instruction_index = u16::from_le_bytes([data[4], data[5]]);
    let public_key_offset = u16::from_le_bytes([data[6], data[7]]) as usize;
    let public_key_instruction_index = u16::from_le_bytes([data[8], data[9]]);
    let message_offset = u16::from_le_bytes([data[10], data[11]]) as usize;
    let message_size = u16::from_le_bytes([data[12], data[13]]) as usize;
    let message_instruction_index = u16::from_le_bytes([data[14], data[15]]);

    // Reject instructions where signature, pubkey, or message are sourced from
    // another instruction (instruction_index != u16::MAX).
    // This prevents instruction-index confusion / attestation forgery bypass.
    if signature_instruction_index != u16::MAX
        || public_key_instruction_index != u16::MAX
        || message_instruction_index != u16::MAX
    {
        return None;
    }

    let sig_end = signature_offset.checked_add(64)?;
    let pk_end = public_key_offset.checked_add(32)?;
    let msg_end = message_offset.checked_add(message_size)?;
    if sig_end > data.len() || pk_end > data.len() || msg_end > data.len() {
        return None;
    }
    let pk = Pubkey::try_from(&data[public_key_offset..pk_end]).ok()?;
    Some((pk, &data[message_offset..msg_end]))
}
#[derive(Accounts)]
#[instruction(platform: u8, platform_uid: String, handle: String, claim_nonce: u64, expires_at: i64, entry_indices: Vec<u64>, mint: Option<Pubkey>, creator_hash: [u8; 16])]
pub struct Claim<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, PlatformConfig>,
    #[account(mut, seeds = [b"treasury"], bump)]
    pub treasury: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"escrow", creator_hash.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, CreatorEscrow>,
    #[account(mut)]
    pub payout: Signer<'info>,
    /// CHECK: sysvar instructions account for Ed25519 introspection
    #[account(address = IX_SYSVAR_ID)]
    pub sysvar_instructions: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
