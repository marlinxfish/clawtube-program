use anchor_lang::prelude::*;

#[error_code]
pub enum ClawtubeError {
    #[msg("Platform is not a valid enum value")]
    InvalidPlatform,
    #[msg("Platform is disabled in the platform registry")]
    PlatformDisabled,
    #[msg("Mint is not whitelisted in ProtocolConfig")]
    MintNotWhitelisted,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Donor balance too low to cover donation, fees, and rent")]
    InsufficientDonorBalance,
    #[msg("Claim attestation expired")]
    AttestationExpired,
    #[msg("Attestation nonce is not greater than the last claimed nonce")]
    NonceReplay,
    #[msg("Oracle signature invalid")]
    InvalidOracleSignature,
    #[msg("Attestation platform does not match escrow platform")]
    PlatformMismatch,
    #[msg("Attestation uid does not match escrow uid")]
    UidMismatch,
    #[msg("Entry is not in a state that allows this operation")]
    InvalidEntryStatus,
    #[msg("Entry is still claimable by the creator")]
    EntryStillClaimable,
    #[msg("Only the donor recorded on the entry can refund it")]
    NotDonor,
    #[msg("Refund window has closed; entry is sweepable")]
    RefundWindowClosed,
    #[msg("Refund window not open yet (day 91-120 only)")]
    RefundWindowNotOpen,
    #[msg("Sweep window not open yet (after day 120)")]
    SweepWindowNotOpen,
    #[msg("Vault ATA owner or mint mismatch with escrow PDA")]
    VaultMismatch,
    #[msg("Vault invariant violated: vault balance below sum of active entries")]
    VaultInvariantViolated,
    #[msg("Payout wallet must sign the claim")]
    PayoutWalletNotSigner,
    #[msg("Whitelist cannot exceed 3 SPL mints")]
    WhitelistOverflow,
    #[msg("Whitelist contains a duplicate or zero mint")]
    DuplicateMint,
    #[msg("Authority or oracle cannot be the zero pubkey")]
    ZeroPubkey,
}
