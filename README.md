# ClawTube Protocol Solana Program

Platform-agnostic decentralized social tipping protocol on Solana (Phase 1: YouTube enabled).

## Overview

ClawTube allows users to tip creators on Web2 platforms (YouTube in Phase 1, expanding to X, Instagram, Threads, TikTok, and Facebook) using native SOL or whitelisted SPL tokens. Creators can claim accumulated tips via oracle-attested dual-signature claims with anti-replay guarantees.

## PDA Architecture

| PDA | Seeds | Purpose |
|-----|-------|---------|
| `PlatformConfig` | `[b"config"]` | Global configuration (authority, oracle, platform registry bitmask, 3 SPL whitelist slots) |
| `Treasury` | `[b"treasury"]` | Fee sink for SOL / SPL protocol fees |
| `Buyback` | `[b"buyback"]` | 70% sweep sink for buyback operations |
| `Ops` | `[b"ops"]` | 28% sweep sink for protocol operations / infrastructure |
| `CreatorEscrow` | `[b"escrow", creator_hash]` | Creator escrow account (`creator_hash = sha256(platform ++ uid)[:16]`) |
| `Entry` | `[b"entry", creator_hash, index_le]` | Individual tip entry recording donor, amount, status, expiry, and sweep windows |

## Instructions

1. `initialize_config(platform_registry: u8, oracle: Option<Pubkey>)` — Initializes the protocol config PDA.
2. `update_authority(new_authority: Pubkey)` — Updates config authority (authority-gated).
3. `set_oracle(oracle: Option<Pubkey>)` — Updates oracle public key for claim attestations.
4. `update_whitelist(mints: Vec<Pubkey>)` — Configures up to 3 whitelisted 6-decimal SPL tokens.
5. `tip_sol(platform: u8, platform_uid: String, amount: u64)` — Tips native SOL (2% protocol fee, 98% escrow).
6. `tip_spl(platform: u8, platform_uid: String, amount: u64)` — Tips whitelisted SPL tokens (2% fee, 98% vault ATA).
7. `claim(platform: u8, platform_uid: String, handle: String, claim_nonce: u64, expires_at: i64, entry_indices: Vec<u64>)` — Creator claims accumulated active tips with Ed25519 oracle attestation (5% claim fee). SPL entries require the vault/payout/treasury ATAs + token program as optional accounts.
8. `refund()` — Donor refunds unclaimed tips during days 91–120.
9. `sweep()` — Permissionless sweep for stale tips after day 120 (70% buyback / 28% ops / 2% sweeper bounty).

## Fee & Lifecycle Schedule

- **Donation Fee**: 200 bps (2%)
- **Claim Fee**: 500 bps (5%)
- **Claim Window**: Day 0–90
- **Refund Window**: Day 91–120 (donor only)
- **Sweep Window**: Day 121+ (permissionless, 70/28/2 split)

## Verification & Testing

```bash
cargo test
cargo check
```
