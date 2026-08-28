use anchor_lang::prelude::*;

pub mod error;
pub mod event;
pub mod state;

pub use error::*;
pub use event::*;
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
}
