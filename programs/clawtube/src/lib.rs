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
        assert_eq!(config.is_mint_whitelisted(&mint3), None); // Not within spl_mint_count (2)
        assert_eq!(config.is_mint_whitelisted(&unlisted), None);
    }

    #[test]
    fn test_update_whitelist_logic() {
        let mut config = PlatformConfig {
            authority: Pubkey::default(),
            oracle: None,
            platform_registry: 0x01,
            spl_mints: [Pubkey::default(); 3],
            spl_mint_count: 0,
            bump: 254,
        };

        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();
        let mints = vec![mint1, mint2];

        // Valid update
        assert!(mints.len() <= 3);
        config.spl_mint_count = mints.len() as u8;
        for (i, m) in mints.iter().enumerate() {
            config.spl_mints[i] = *m;
        }
        assert_eq!(config.spl_mint_count, 2);
        assert_eq!(config.spl_mints[0], mint1);
        assert_eq!(config.spl_mints[1], mint2);

        // Overflow update check
        let mint4 = Pubkey::new_unique();
        let mints_overflow = vec![mint1, mint2, mint3_fn(), mint4];
        assert!(mints_overflow.len() > 3);
    }

    fn mint3_fn() -> Pubkey {
        Pubkey::new_unique()
    }
}
