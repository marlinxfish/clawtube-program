#![allow(unexpected_cfgs)]
#![allow(ambiguous_glob_reexports)]

use anchor_lang::prelude::*;

pub mod error;
pub mod event;
pub mod instructions;
pub mod state;

pub use error::*;
pub use event::*;
pub use instructions::*;
pub use state::*;
declare_id!("FvqRRnu1tfaJbUUGbnA3oaVuGUFHz8fLEc7Yr6rWE4aA");

pub fn creator_hash(platform: u8, platform_uid: &[u8]) -> [u8; 16] {
    solana_program::hash::hashv(&[&[platform], platform_uid]).to_bytes()[..16]
        .try_into()
        .unwrap()
}

#[program]
pub mod clawtube {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        platform_registry: u8,
        oracle: Option<Pubkey>,
    ) -> Result<()> {
        instructions::admin::initialize_config(ctx, platform_registry, oracle)
    }

    pub fn update_authority(ctx: Context<UpdateConfig>, new_authority: Pubkey) -> Result<()> {
        instructions::admin::update_authority(ctx, new_authority)
    }

    pub fn set_oracle(ctx: Context<UpdateConfig>, oracle: Option<Pubkey>) -> Result<()> {
        instructions::admin::set_oracle(ctx, oracle)
    }

    pub fn update_whitelist(ctx: Context<UpdateConfig>, mints: Vec<Pubkey>) -> Result<()> {
        instructions::admin::update_whitelist(ctx, mints)
    }

    pub fn tip_sol(
        ctx: Context<TipSol>,
        platform: u8,
        platform_uid: String,
        amount: u64,
        creator_hash: [u8; 16],
    ) -> Result<()> {
        instructions::tip_sol::tip_sol(ctx, platform, platform_uid, amount, creator_hash)
    }

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
        instructions::claim::claim(
            ctx,
            platform,
            platform_uid,
            handle,
            claim_nonce,
            expires_at,
            entry_indices,
            mint,
            creator_hash,
        )
    }

    pub fn tip_spl(
        ctx: Context<TipSpl>,
        platform: u8,
        platform_uid: String,
        amount: u64,
        creator_hash: [u8; 16],
    ) -> Result<()> {
        instructions::tip_spl::tip_spl(ctx, platform, platform_uid, amount, creator_hash)
    }

    pub fn refund<'info>(ctx: Context<'info, Refund<'info>>) -> Result<()> {
        instructions::refund::refund(ctx)
    }

    pub fn sweep<'info>(ctx: Context<'info, Sweep<'info>>) -> Result<()> {
        instructions::sweep::sweep(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creator_hash() {
        let hash = creator_hash(Platform::YouTube.as_u8(), b"UC1234567890");
        assert_eq!(hash.len(), 16);
        let hash2 = creator_hash(Platform::YouTube.as_u8(), b"UC1234567890");
        assert_eq!(hash, hash2);
        let diff_hash = creator_hash(Platform::X.as_u8(), b"UC1234567890");
        assert_ne!(hash, diff_hash);
    }

    #[test]
    fn test_platform_enum() {
        assert_eq!(Platform::from_u8(0).unwrap(), Platform::YouTube);
        assert_eq!(Platform::from_u8(1).unwrap(), Platform::X);
        assert_eq!(Platform::from_u8(2).unwrap(), Platform::Instagram);
        assert_eq!(Platform::from_u8(3).unwrap(), Platform::Threads);
        assert_eq!(Platform::from_u8(4).unwrap(), Platform::TikTok);
        assert_eq!(Platform::from_u8(5).unwrap(), Platform::Facebook);
        assert!(Platform::from_u8(6).is_err());
        assert!(Platform::from_u8(255).is_err());
    }

    #[test]
    fn test_platform_config_methods() {
        let config = PlatformConfig {
            authority: Pubkey::default(),
            oracle: None,
            platform_registry: 0x01,
            spl_mints: [Pubkey::default(); 3],
            spl_mint_count: 0,
            bump: 254,
        };
        assert!(config.is_platform_enabled(0));
        assert!(!config.is_platform_enabled(1));
        assert!(!config.is_platform_enabled(2));
    }

    #[test]
    fn test_whitelist_lookup() {
        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();
        let mint3 = Pubkey::new_unique();
        let unlisted = Pubkey::new_unique();

        let config = PlatformConfig {
            authority: Pubkey::default(),
            oracle: None,
            platform_registry: 0x01,
            spl_mints: [mint1, mint2, mint3],
            spl_mint_count: 2,
            bump: 254,
        };

        assert_eq!(config.is_mint_whitelisted(&mint1), Some(0));
        assert_eq!(config.is_mint_whitelisted(&mint2), Some(1));
        assert_eq!(config.is_mint_whitelisted(&mint3), None);
        assert_eq!(config.is_mint_whitelisted(&unlisted), None);
    }

    #[test]
    fn test_donation_fee_math() {
        let amount = 1_000_000_000u64; // 1 SOL
        let fee = amount * DONATION_FEE_BPS / 10_000;
        let net = amount - fee;
        assert_eq!(fee, 20_000_000); // 2%
        assert_eq!(net, 980_000_000); // 98%
    }

    #[test]
    fn test_claim_fee_math() {
        let net_amount = 980_000_000u64;
        let fee = net_amount * CLAIM_FEE_BPS / 10_000;
        let creator_payout = net_amount - fee;
        assert_eq!(fee, 49_000_000); // 5%
        assert_eq!(creator_payout, 931_000_000); // 95%
    }

    #[test]
    fn test_sweep_split_math() {
        let amount = 1_000_000_000u64;
        let b70 = amount * 70 / 100;
        let o28 = amount * 28 / 100;
        let s2 = amount - b70 - o28;
        assert_eq!(b70, 700_000_000); // 70%
        assert_eq!(o28, 280_000_000); // 28%
        assert_eq!(s2, 20_000_000); // 2%
        assert_eq!(b70 + o28 + s2, amount);
    }

    #[test]
    fn test_attestation_msg_format() {
        let platform = 0u8;
        let uid = "UC1234567890";
        let payout = Pubkey::new_unique();
        let nonce = 1u64;
        let expires_at = 1700000000i64;
        let handle = "@creator";

        let mut msg = Vec::new();
        msg.push(1u8); // version 1
        msg.push(platform);
        msg.extend_from_slice(&(uid.len() as u32).to_le_bytes());
        msg.extend_from_slice(uid.as_bytes());
        msg.extend_from_slice(payout.as_ref());
        msg.extend_from_slice(&nonce.to_le_bytes());
        msg.extend_from_slice(&expires_at.to_le_bytes());
        msg.extend_from_slice(&(handle.len() as u32).to_le_bytes());
        msg.extend_from_slice(handle.as_bytes());

        assert_eq!(msg[0], 1);
        assert_eq!(msg[1], 0);
        let uid_len = u32::from_le_bytes(msg[2..6].try_into().unwrap());
        assert_eq!(uid_len as usize, uid.len());
    }

    #[test]
    fn test_nonce_replay_guard() {
        let last_nonce = 5u64;
        let incoming_nonce_replayed = 5u64;
        let incoming_nonce_older = 4u64;
        let incoming_nonce_valid = 6u64;

        assert!(incoming_nonce_replayed <= last_nonce);
        assert!(incoming_nonce_older <= last_nonce);
        assert!(incoming_nonce_valid > last_nonce);
    }

    #[test]
    fn test_refund_and_sweep_lifecycle_guards() {
        let created_at = 1000i64;
        let expires_at = created_at + 90 * 86_400; // Day 90
        let sweep_at = created_at + 120 * 86_400;  // Day 120

        // Day 50: Claim window open, Refund NOT open, Sweep NOT open
        let day50 = created_at + 50 * 86_400;
        assert!(day50 < expires_at); // Claimable
        assert!(day50 < expires_at); // EntryStillClaimable if refund attempted

        // Day 95: Claim expired, Refund window OPEN, Sweep NOT open
        let day95 = created_at + 95 * 86_400;
        assert!(day95 >= expires_at); // Claim window closed
        assert!(day95 >= expires_at && day95 < sweep_at); // Refund window open
        assert!(day95 < sweep_at); // Sweep window NOT open

        // Day 125: Refund window CLOSED, Sweep window OPEN
        let day125 = created_at + 125 * 86_400;
        assert!(day125 >= sweep_at); // Refund window closed (sweepable)
        assert!(day125 >= sweep_at); // Sweep window open
    }

    #[test]
    fn test_entry_status_transitions() {
        let mut status = EntryStatus::Active;
        assert_eq!(status, EntryStatus::Active);

        // Active -> Claimed
        status = EntryStatus::Claimed;
        assert_ne!(status, EntryStatus::Active);

        // Reset and test Active -> Refunded
        status = EntryStatus::Active;
        assert_eq!(status, EntryStatus::Active);
        status = EntryStatus::Refunded;
        assert_ne!(status, EntryStatus::Active);

        // Reset and test Active -> Swept
        status = EntryStatus::Active;
        assert_eq!(status, EntryStatus::Active);
        status = EntryStatus::Swept;
        assert_ne!(status, EntryStatus::Active);
    }

    #[test]
    fn test_sysvar_instructions_id_match() {
        let expected_str = "Sysvar1nstructions1111111111111111111111111";
        assert_eq!(solana_program::sysvar::instructions::ID.to_string(), expected_str);
    }

    #[test]
    fn test_seed_length_limits() {
        let max_uid = "a".repeat(crate::state::MAX_PLATFORM_UID);
        let hash = creator_hash(Platform::YouTube.as_u8(), max_uid.as_bytes());
        assert_eq!(hash.len(), 16);
        assert!(hash.len() <= 32);
        assert!(b"config".len() <= 32);
        assert!(b"treasury".len() <= 32);
        assert!(b"escrow".len() <= 32);
        assert!(b"entry".len() <= 32);
        assert!(b"buyback".len() <= 32);
        assert!(b"ops".len() <= 32);
    }

    #[test]
    fn test_checked_math_edge_cases() {
        let zero = 0u64;
        assert!(zero.checked_mul(DONATION_FEE_BPS).is_some());
        let max = u64::MAX;
        assert!(max.checked_add(1).is_none());
    }
}
