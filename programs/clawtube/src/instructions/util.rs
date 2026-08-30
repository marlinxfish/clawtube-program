use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};
use crate::error::ClawtubeError;

/// Deserialize and validate that `account_info` is a token account whose
/// mint and authority match the expected values. The token program owner is
/// also checked to reject accounts that are not SPL token accounts.
pub fn require_token_account<'info>(
    account_info: &AccountInfo<'info>,
    token_program: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
) -> Result<()> {
    require_keys_eq!(
        *account_info.owner,
        *token_program,
        ClawtubeError::VaultMismatch
    );
    let data = account_info.try_borrow_data()?;
    let acc = TokenAccount::try_deserialize(&mut &data[..])?;
    require!(acc.mint == *mint, ClawtubeError::VaultMismatch);
    require!(acc.owner == *authority, ClawtubeError::VaultMismatch);
    Ok(())
}

/// Transfer `amount` tokens from the escrow vault ATA to `dest`, signed by
/// the escrow PDA (the vault's authority). The escrow PDA is derived from
/// `[b"escrow", escrow_hash]` with `escrow_bump`.
pub fn transfer_spl_from_vault<'info>(
    token_program: &Program<'info, Token>,
    vault: &AccountInfo<'info>,
    dest: &AccountInfo<'info>,
    escrow: &AccountInfo<'info>,
    escrow_hash: [u8; 16],
    escrow_bump: u8,
    amount: u64,
) -> Result<()> {
    let seeds: &[&[u8]] = &[b"escrow".as_ref(), escrow_hash.as_ref(), &[escrow_bump]];
    let signer: &[&[&[u8]]] = &[seeds];
    transfer(
        CpiContext::new_with_signer(
            token_program.key(),
            Transfer {
                from: vault.clone(),
                to: dest.clone(),
                authority: escrow.clone(),
            },
            signer,
        ),
        amount,
    )
}
