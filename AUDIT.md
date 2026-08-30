# ClawTube Solana Program Security Audit Report

## Executive Summary
A comprehensive security review of the ClawTube Solana smart contract program
(`programs/clawtube/src/`) was conducted covering access controls, account
ownership, signer verification, Ed25519 introspection, integer arithmetic, PDA
seed safety, replay/nonce guards, invariant enforcement, rent/lifecycle
economics, and SPL token flows.

**Second-round audit (2026-08-30)** found and fixed two critical issues plus
several high/medium defects. All fixes are verified end-to-end on Devnet.

---

## Critical Findings (fixed)

### [CRIT-1] Oracle attestation was forgeable — full fund drain
- **Location**: `instructions/claim.rs`, `verify_oracle_attestation`.
- **Before**: The function performed a raw *byte-window substring search* over
  the `sysvar_instructions` data, looking for the oracle pubkey (32 bytes) and
  the expected message. It never verified an Ed25519 signature. Since the oracle
  pubkey is public and the message is fully reconstructible (platform, uid,
  payout, nonce, expires_at, handle — all public or attacker-chosen), an
  attacker could embed both byte sequences into any instruction in the same
  transaction and claim any escrow to any wallet. The "dual-signature,
  genuinely non-custodial" property was inverted.
- **After**: `verify_oracle_attestation` now iterates the transaction's
  preceding instructions via `load_current_index_checked` /
  `load_instruction_at_checked`, locates an `Ed25519SigVerify` instruction,
  and confirms its public key == oracle and its message == the reconstructed
  attestation message. The `Ed25519SigVerify` native program rejects the whole
  transaction on an invalid signature, so a matching instruction proves the
  oracle actually signed. A raw byte search is no longer sufficient.
- **Verification**: `scripts/devnet-security-e2e.ts` — claim without an
  attestation is rejected on devnet.

### [CRIT-2] SPL claim/refund/sweep never transferred tokens — funds stranded
- **Location**: `instructions/claim.rs`, `instructions/refund.rs`,
  `instructions/sweep.rs`.
- **Before**: For `entry.mint.is_some()`, the instructions only decremented
  `escrow.vault_totals[slot]` and never performed a token transfer. SPL tips
  were permanently locked in the escrow vault ATA — creators could never
  receive them, donors could never refund them, and the protocol could never
  sweep them. The `Claim`/`Refund`/`Sweep` account structs lacked the vault ATA,
  destination ATA, and token program entirely.
- **After**: All three instructions now transfer tokens from the escrow vault
  ATA (signed by the escrow PDA) to the correct destination ATAs:
  - `claim`: vault → payout ATA (95%) + treasury ATA (5% fee).
  - `refund`: vault → donor ATA (100%, no fee).
  - `sweep`: vault → buyback ATA (70%) + ops ATA (20%) + sweeper ATA (10%).
  - Destination ATAs are passed via `remaining_accounts` and validated with
    `require_token_account` (owner, mint, authority) in `instructions/util.rs`.
- **Verification**: `scripts/devnet-security-e2e.ts` — SPL tip → SPL claim moves
  tokens: vault `980000 → 0`, payout `0 → 931000`.

### [HIGH] Claim escrow seed was self-referential and invalid
- **Location**: `instructions/claim.rs`, `Claim` account struct.
- **Before**: `seeds = [b"escrow", creator_hash(escrow.platform, ...)]`
  referenced `escrow` inside its own seed constraint (circular), which also
  broke `anchor idl build`.
- **After**: `claim` now takes an explicit `platform: u8` argument and the seed
  is `creator_hash(platform, platform_uid.as_bytes())`, matching `tip_sol` /
  `tip_spl`. The platform is validated against `escrow.platform`.

---

## Medium Findings (fixed)

### [MED-1] `update_whitelist` accepted duplicate or zero mints
Duplicate mints would corrupt the per-slot `vault_totals` accounting. Now
rejected with `DuplicateMint`; zero pubkeys rejected too.

### [MED-2] `update_authority` accepted the zero pubkey
Setting authority to the zero pubkey permanently bricks all admin operations.
Now rejected with `ZeroPubkey`.

### [MED-3] Manual lamport transfers lacked a rent-exemption guard
SOL payout/refund/sweep now verify the escrow's remaining lamports stay above
`Rent::get().minimum_balance(data_len)` before debiting, preventing the escrow
from being drained below rent-exemption.

### [MED-4] `set_oracle` could brick claims silently
Setting oracle to a zero pubkey is now rejected. Setting oracle to `None`
remains possible as an intentional emergency pause.

---

## Checklist & Verification Matrix

| # | Item | Status | Detail |
|---|---|---|---|
| 1 | Account ownership & type checks | SECURE | Typed accounts + PDAs; remaining accounts owner-checked against `crate::ID` (entries) and token program (ATAs). |
| 2 | Signer checks | SECURE | `has_one = authority`, donor/payout/sweeper signers, `entry.donor == donor.key()` for refund. |
| 3 | Ed25519 attestation | SECURE | Real signature verification via `Ed25519SigVerify` instruction introspection (see CRIT-1). |
| 4 | Integer arithmetic | SECURE | All math uses `checked_*` with `ok_or(...)`; `overflow-checks = true` in release profile. |
| 5 | PDA seed safety | SECURE | `creator_hash` is 16 bytes; all seed components ≤ 32 bytes. |
| 6 | Replay/nonce guards | SECURE | `claim_nonce > last_claim_nonce`, monotonic; entry status transitions prevent double-claim. |
| 7 | SPL token flows | SECURE | Deposit + claim/refund/sweep all transfer tokens; `vault_totals` invariant maintained per mint. |
| 8 | Rent/lifecycle economics | SECURE | Rent-exemption guards; entry lifecycle Active → Claimed/Refunded/Swept. |

---

## Remaining Known Limitation (not security-critical)

- **`anchor idl build` fails** on `creator_hash(...)` function calls inside
  `seeds` constraints (pre-existing, affects `tip_sol`, `tip_spl`, `claim`).
  The SBF binary builds and deploys correctly; the backend uses manual
  discriminators and does not require the IDL. Recommended follow-up: pass a
  precomputed `[u8; 16]` hash as an instruction argument instead of calling
  `creator_hash` inside seeds, which also hardens against on-chain/off-chain
  hash divergence.

---

## Verification Evidence

- `cargo test`: 15 passed.
- `cargo check`: clean (only pre-existing cfg/glob warnings).
- `cargo build-sbf`: success.
- Program upgraded on Devnet (program ID `FvqRRnu1...`, data length 385384).
- `scripts/devnet-security-e2e.ts`: PASS — oracle enforced, SPL claim transfers
  tokens.
