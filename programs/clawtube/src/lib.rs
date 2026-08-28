//! ClawTube Protocol — Decentralized Web2-to-Web3 Social Tipping on Solana
//!
//! Platform-agnostic tipping: YouTube (fase 1), X/Instagram/Threads/TikTok/Facebook (fase 2).
//! Desain lengkap: docs/superpowers/specs/2026-08-28-multi-platform-design.md di repo workspace utama.
//!
//! PDA skema (seed ≤ 32 byte):
//!   creator_hash  = sha256(platform_u8 ++ platform_uid_bytes)[:16]
//!   Escrow PDA   = [b"escrow", creator_hash]           (22 byte)
//!   Entry PDA    = [b"entry", creator_hash, index_le]  (30 byte)

use anchor_lang::prelude::*;

declare_id!("ClawTubeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee1");

/// Platform whitelist on-chain (enum tertutup; platform baru = program upgrade).
#[repr(u8)]
pub enum Platform {
    YouTube = 0,
    X = 1,
    Instagram = 2,
    Threads = 3,
    TikTok = 4,
    Facebook = 5,
}

// 8 instruksi protokol (implementasi menyusul via writing-plans):
// 1.  initialize_config  — ProtocolConfig PDA: whitelist mint (max 3), oracle pubkey, platform bitmask
// 2.  tip_sol            — donasi native SOL: 2% treasury, 98% ke escrow PDA
// 3.  tip_spl            — donasi SPL ($TDROP/$CLAW): sama, vault ATA per token (lazy init)
// 4.  claim              — klaim dual-signature (oracle attestasi + payout wallet kreator), fee 5%
// 5.  refund             — refund donatur jendela hari 91–120 (terikat signature donatur)
// 6.  sweep              — sweep entri >120 hari ke treasury (bounty 10% + sewa, permissionless)
//
// Fee konstanta: DONATION_FEE_BPS = 200, CLAIM_FEE_BPS = 500 (baked const, tanpa runtime setter).

#[program]
pub mod clawtube {
    use super::*;

    // Instruksi diimplementasi bertahap sesuai implementation plan.
}
