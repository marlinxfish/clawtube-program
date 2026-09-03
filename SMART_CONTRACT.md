# ClawTube Smart Contract Architecture

This document details the architecture and operational mechanics of the ClawTube Solana smart contract: on-chain account structures, instructions, **fee flows across donors, creators, and protocol**, and timelock lifecycle transitions. Constant values are defined in `programs/clawtube/src/state.rs`.

---

## 1. On-Chain Accounts

### 1.1 `PlatformConfig`
- **Seeds**: `[b"config"]`
- **Size**: 172 bytes
- Global protocol settings: Authority, Oracle Pubkey, Enabled Platform bitmask, Whitelisted SPL Mints (up to 3 mints).

### 1.2 `CreatorEscrow`
- **Seeds**: `[b"escrow", creator_hash]`
- **Size**: 203 bytes
- `creator_hash = sha256(platform ++ platform_uid)[:16]` — cryptographically binds the escrow vault to the creator identity, preventing collisions across creators.
- Tracks `entry_count`, `last_claim_nonce`, `handle`, and per-mint token totals (`vault_totals: [u64; 3]`).

### 1.3 `Entry`
- **Seeds**: `[b"entry", creator_hash, index.to_le_bytes()]`
- **Size**: 123 bytes
- Immutable record per donation: donor wallet, amount, mint (`None` for SOL), status (`Active`, `Claimed`, `Refunded`, `Swept`), `expires_at` (Day 90), `sweep_at` (Day 120).

---

## 2. Protocol Instructions

1. `initialize_config` — Deploys protocol configuration and assigns authority.
2. `update_authority` — Transfers admin authority to a new keypair.
3. `set_oracle` — Updates or pauses oracle attestation key.
4. `update_whitelist` — Configures up to 3 whitelisted SPL tokens.
5. `tip_sol` — Donates native SOL to creator escrow vault (0% fee, 100% to creator escrow).
6. `tip_spl` — Donates whitelisted SPL tokens to creator vault ATA (0% fee, 100% to creator escrow).
8. `refund` — 100% donor refund during Days 91–120.
9. `sweep` — Permissionless liquidation after Day 120 (70% Buyback, 28% Ops, 2% Sweeper bounty).
10. `withdraw_fees` & `withdraw_fees_spl` — Authority disbursement of protocol treasury/swept funds.

---

## 3. Financial Flows & Fee Invariants

### 3.1 Donor Deposit (Tip)
- **0% Tip Fee** (no deduction at deposit time).
- **100% Net Amount** deposited into the creator escrow vault.

### 3.2 Creator Claim (Days 0–90)
- **5% Claim Fee** deducted from claimed amount and forwarded to Treasury PDA.
- **95% Net Amount** transferred directly to creator payout wallet.
- Requires dual authorization: Ed25519 Oracle attestation signature + Creator payout wallet signature.

### 3.3 Donor Refund (Days 91–120)
- **100% Nominal Refund** (no claim fee deducted).
- Only the recorded donor wallet can execute refund.

### 3.4 Sweeper Liquidation (Days 121+)
- **70%** routed to $CLAWTUBE open market buyback & burn.
- **28%** routed to protocol operational infrastructure.
- **2%** awarded as immediate bounty to the permissionless sweeper caller.

---

## 4. Security Model & Guarantees
- **Non-Custodial**: Neither server nor oracle has private key custody.
- **Strict Ed25519 Introspection**: On-chain verification confirms opcode indices match the local instruction data (`0xFFFF`), blocking instruction-index confusion.
- **Anti-Replay Monotonic Nonces**: Every claim consumes a strictly incrementing `claim_nonce`.
- **Checked Arithmetic**: All token and lamport calculations use `checked_*` with zero-overflow tolerance.
- **Dual-Signature Barrier**: Oracle attestation alone cannot drain funds without the creator's payout wallet signature.
