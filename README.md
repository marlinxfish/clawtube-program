# clawtube-program

Anchor smart contract **ClawTube Protocol** — social tipping terdesentralisasi YouTube → Web3 di Solana.

## Stack
- Rust + Anchor 1.1.2 (Solana CLI 3.1.10)
- Test: LiteSVM / Surfpool

## Struktur
```
programs/clawtube/src/lib.rs   # deklarasi program, enum Platform, skema PDA
tests/                          # suite test Anchor
.agents/skills/solana-dev/     # skill Solana dev (agent-agnostic)
.claude/skills/solana-dev/     # skill Solana dev (Claude Code)
```

## Build & Test
```bash
anchor build
anchor test
```

Spesifikasi desain lengkap (multi-platform, skema PDA hash 16-byte, keamanan): lihat repo workspace utama `docs/superpowers/specs/2026-08-28-multi-platform-design.md`.
