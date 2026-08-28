use anchor_lang::prelude::*;

pub mod error;
pub mod event;
pub mod instructions;
pub mod state;

pub use error::*;
pub use event::*;
pub use instructions::*;
pub use state::*;

declare_id!("CLawTubeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee1");

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
    ) -> Result<()> {
        instructions::tip_sol::tip_sol(ctx, platform, platform_uid, amount)
    }

    pub fn claim<'info>(
        ctx: Context<'info, Claim<'info>>,
        platform_uid: String,
        handle: String,
        claim_nonce: u64,
        expires_at: i64,
        entry_indices: Vec<u64>,
    ) -> Result<()> {
        instructions::claim::claim(
            ctx,
            platform_uid,
            handle,
            claim_nonce,
            expires_at,
            entry_indices,
        )
    }

    pub fn tip_spl(
        ctx: Context<TipSpl>,
        platform: u8,
        platform_uid: String,
        amount: u64,
    ) -> Result<()> {
        instructions::tip_spl::tip_spl(ctx, platform, platform_uid, amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creator_hash() {
        let hash = creator_hash(Platform::YouTube.as_u8(), b"UC1234567890");
        assert_eq!(hash.len(), 16);
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
}
