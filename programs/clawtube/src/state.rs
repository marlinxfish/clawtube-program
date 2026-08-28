use anchor_lang::prelude::*;

pub const DONATION_FEE_BPS: u64 = 200;
pub const CLAIM_FEE_BPS: u64 = 500;
pub const MAX_PLATFORM_UID: usize = 64;
pub const MAX_HANDLE: usize = 64;

#[account]
#[derive(Debug, PartialEq)]
pub struct PlatformConfig {
    pub authority: Pubkey,
    pub oracle: Option<Pubkey>,
    pub platform_registry: u8,
    pub spl_mints: [Pubkey; 3],
    pub spl_mint_count: u8,
    pub bump: u8,
}

impl PlatformConfig {
    pub const SIZE: usize = 8 + 32 + 1 + 32 + 1 + 96 + 1 + 1; // 172
    pub fn is_platform_enabled(&self, platform: u8) -> bool {
        (self.platform_registry >> platform) & 1 == 1
    }
    pub fn is_mint_whitelisted(&self, mint: &Pubkey) -> Option<usize> {
        self.spl_mints[..self.spl_mint_count as usize]
            .iter()
            .position(|m| m == mint)
    }
}

#[account]
#[derive(Debug, PartialEq)]
pub struct CreatorEscrow {
    pub creator_hash: [u8; 16],
    pub platform: u8,
    pub platform_uid: String,
    pub handle: String,
    pub vault_bump: u8,
    pub entry_count: u64,
    pub last_claim_nonce: u64,
    pub vault_totals: [u64; 3],
    pub bump: u8,
}

impl CreatorEscrow {
    pub const SIZE: usize = 8 + 16 + 1 + 4 + MAX_PLATFORM_UID + 4 + MAX_HANDLE + 1 + 8 + 8 + 24 + 1;
}

#[account]
#[derive(Debug, PartialEq)]
pub struct Entry {
    pub creator_hash: [u8; 16],
    pub index: u64,
    pub amount: u64,
    pub mint: Option<Pubkey>,
    pub donor: Pubkey,
    pub status: EntryStatus,
    pub expires_at: i64,
    pub sweep_at: i64,
    pub bump: u8,
}

impl Entry {
    pub const SIZE: usize = 8 + 16 + 8 + 8 + 1 + 32 + 32 + 1 + 8 + 8 + 1;
}

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Platform {
    YouTube,
    X,
    Instagram,
    Threads,
    TikTok,
    Facebook,
}

impl Platform {
    pub fn from_u8(v: u8) -> Result<Self> {
        match v {
            0 => Ok(Self::YouTube),
            1 => Ok(Self::X),
            2 => Ok(Self::Instagram),
            3 => Ok(Self::Threads),
            4 => Ok(Self::TikTok),
            5 => Ok(Self::Facebook),
            _ => Err(crate::error::ClawtubeError::InvalidPlatform.into()),
        }
    }
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntryStatus {
    Active,
    Claimed,
    Refunded,
    Swept,
}
