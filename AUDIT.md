# ClawTube Solana Program Security Audit Report

## Executive Summary
A comprehensive security review of the ClawTube Solana smart contract program (`programs/clawtube/src/`) was conducted covering access controls, account ownership, signer verification, Ed25519 introspection, integer arithmetic, PDA seed safety, replay/nonce guards, invariant enforcement, and rent/lifecycle economics.

---

## Audit Checklist & Verification Matrix

| Checklist Item | Description | Status | Details |
|---|---|---|---|
| **1. Account Ownership & Type Checks** | Are all accounts properly typed, owned, and constrained? | **SECURE** | All accounts utilize Anchor's typed `Account<'info, T>`, `Signer<'info>`, `Program<'info, System>`, `Program<'info, Token>`, and checked PDAs. Added explicit owner verification `entry_account_info.owner == &crate::ID` and strict address checking for sysvar instructions. |
| **2. Signer Checks** | Are all mutating authorities required signers? | **SECURE** | `InitializeConfig` and `UpdateConfig` enforce authority signers with `has_one = authority`. `TipSol` and `TipSpl` enforce donor signatures. `Claim` enforces payout wallet signature. `Refund` verifies donor signer match against entry donor. `Sweep` enforces sweeper signer. |
| **3. Ed25519 Introspection** | Is Oracle signature verified against expected serialized message and sysvar instructions? | **SECURE** | Hardened `verify_oracle_attestation` and `Claim` context to bind `sysvar_instructions` strictly to `Sysvar1nstructions1111111111111111111111111`. Replaced permissive empty data path with strict non-empty data requirement and exact byte window search. |
| **4. Integer Overflow / Underflow** | Are all math operations using checked arithmetic? | **SECURE** | All addition, subtraction, multiplication, and division operations use `checked_add`, `checked_sub`, `checked_mul`, and `checked_div` with custom Anchor error propagation (`ok_or(...)`). |
| **5. PDA Seed Constraints** | Total seed length <= 32 bytes per seed component? | **SECURE** | `creator_hash = sha256(platform ++ uid)[:16]` (16 bytes). Seed prefixes (`b"config"`, `b"treasury"`, `b"escrow"`, `b"entry"`, `b"buyback"`, `b"ops"`) are all between 3 and 8 bytes. Entry index seed is 8 bytes (`u64::to_le_bytes`). All seeds <= 32 bytes. |
| **6. Replay & Nonce Attacks** | Is `last_claim_nonce` stored and monotonically verified? | **SECURE** | `Claim` enforces `claim_nonce > escrow.last_claim_nonce`, serializes the nonce into the oracle attestation message, and records `escrow.last_claim_nonce = claim_nonce`. Active entries transition to `Claimed` status preventing double-claims. |
| **7. Invariant Checks** | Are token/SOL vault balances verified after transfers? | **SECURE** | Lamport transfers out of `escrow` explicitly check `checked_sub` against current escrow lamports, preserving rent-exemption. SPL balances track `vault_totals` per whitelisted mint slot. |
| **8. Griefing & Account Closure** | Is rent returned correctly without destroying non-closable accounts? | **SECURE** | Non-closable state accounts (`PlatformConfig`, `CreatorEscrow`, `Entry`) are never closed prematurely. Entry states transition cleanly across lifecycle: `Active` -> `Claimed` / `Refunded` / `Swept`. |

---

## Findings & Remediations Applied

1. **[SEC-01] Missing Address Constraint on `sysvar_instructions` in `Claim`**:
   - *Impact*: Previously `sysvar_instructions` was typed as `AccountInfo<'info>` with permissive check when data was empty.
   - *Remediation*: Added `#[account(address = solana_program::sysvar::instructions::ID)]` constraint in `Claim` struct and reinforced `require_keys_eq!` and `!data.is_empty()` check inside `verify_oracle_attestation`.
2. **[SEC-02] Missing Program Ownership Verification in Remaining Accounts**:
   - *Impact*: Remaining accounts passed during `claim` for entry iteration could theoretically have bypassed Anchor owner checks if an unowned account matched discriminator.
   - *Remediation*: Enforced `require_keys_eq!(*entry_account_info.owner, crate::ID)` alongside expected PDA check.
3. **[SEC-03] Claim Expiration Invariant on Individual Entries**:
   - *Impact*: Enforced strict entry expiration check `clock.unix_timestamp < entry.expires_at` alongside attestation message expiry check.
4. **[SEC-04] Math Safety & Error Handling**:
   - *Impact*: Replaced all `.unwrap()` calls in checked math paths across `tip_sol`, `tip_spl`, `refund`, `claim`, and `sweep` with Anchor `ok_or(...)` error propagation.
