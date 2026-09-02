use anchor_lang::prelude::*;

#[event]
pub struct TipReceived {
    pub platform: u8,
    pub creator_hash: [u8; 16],
    pub index: u64,
    pub amount: u64,
    pub mint: Option<Pubkey>,
    pub donor: Pubkey,
}

#[event]
pub struct Claimed {
    pub creator_hash: [u8; 16],
    pub index: u64,
    pub amount: u64,
    pub mint: Option<Pubkey>,
    pub payout: Pubkey,
    pub claim_nonce: u64,
}

#[event]
pub struct Refunded {
    pub creator_hash: [u8; 16],
    pub index: u64,
    pub amount: u64,
    pub mint: Option<Pubkey>,
    pub donor: Pubkey,
}

#[event]
pub struct Swept {
    pub creator_hash: [u8; 16],
    pub index: u64,
    pub amount: u64,
    pub mint: Option<Pubkey>,
    pub sweeper: Pubkey,
}

#[event]
pub struct FeesWithdrawn {
    pub pool: u8,
    pub amount: u64,
    pub mint: Option<Pubkey>,
    pub recipient: Pubkey,
}
