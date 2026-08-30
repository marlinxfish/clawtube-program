# ClawTube Protocol — On-Chain Information

Informasi alamat on-chain untuk branch **devnet** (testnet Solana). Nilai mainnet akan diisi saat program di-deploy ke mainnet (branch `main`).

## Cluster

| Item | Nilai |
| :--- | :--- |
| Network | `devnet` |
| RPC | `https://api.devnet.solana.com` |
| Program Loader | `BPFLoaderUpgradeable` (upgradable) |

## Smart Contract (Program)

| Item | Address |
| :--- | :--- |
| **Program ID** | `FvqRRnu1tfaJbUUGbnA3oaVuGUFHz8fLEc7Yr6rWE4aA` |
| Upgrade Authority | `6pnj24Nk5M8PvntXqw4ZPqjXeQoBo7ZCxSUoVm8Cn1Ct` |
| Config Authority | `6pnj24Nk5M8PvntXqw4ZPqjXeQoBo7ZCxSUoVm8Cn1Ct` |
| Oracle (attestation) | `AbvVdJsfagQPAMhSR8tcHYRbQmTLyrNaoNLvVF6pNTDr` |

## PDA Addresses

| PDA | Seed | Address |
| :--- | :--- | :--- |
| ProtocolConfig | `["config"]` | `ENbuR5f23crFWCEcuRf9T2VHrwH3KXRgXLKguDLWiwGt` |
| Treasury | `["treasury"]` | `9GLZCP2C1bxyu5yup5JaP5stv9NsWaAiidrmACQjaj3h` |
| Buyback Treasury | `["buyback"]` | `EFBLDLmnL2DGxDEYjLdWCAheYXUpnwfjiL6CHNT8pwZn` |
| Ops Treasury | `["ops"]` | `76vbcDrvPLfxqjv7uuTzEvYqfcYjNyuizECYN1RNitM5` |
| Creator Escrow | `["escrow", creator_hash]` | per-kanal (deterministik) |
| Donation Entry | `["entry", creator_hash, index]` | per-donasi (deterministik) |

## Whitelisted Tokens (SPL)

| Token | Symbol | Mint Address | Decimals |
| :--- | :--- | :--- | :--- |
| ClawTube | `CLAWTUBE` | `3BDw2BVdpPxHf4uf2yLRgQbhMbnzEEsch9CRScu4otwg` | 6 |
| Claw | `CLAW` | `GEfEtWBGr8LpEc8PEyDtef4pYiewFjQ5e1srfbQTNSVm` | 6 |
| AnsemHack | `ANSEMHACK` | `2TDdv5xYKQoPXN5JTs95DJRtxT3Hztf45BE9xdydfbJ6` | 6 |
| Solana (native) | `SOL` | — | 9 |

Token Program: `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`

## Wallets (Devnet Operator)

| Peran | Address |
| :--- | :--- |
| Deployer / Authority | `6pnj24Nk5M8PvntXqw4ZPqjXeQoBo7ZCxSUoVm8Cn1Ct` |
| Oracle / Faucet | `AbvVdJsfagQPAMhSR8tcHYRbQmTLyrNaoNLvVF6pNTDr` |

## Config Registry (on-chain `ProtocolConfig`)

| Field | Nilai |
| :--- | :--- |
| `platform_registry` | `0x01` (hanya YouTube aktif) |
| `spl_mint_count` | `3` |
| `oracle` | `AbvVdJsfagQPAMhSR8tcHYRbQmTLyrNaoNLvVF6pNTDr` |
| `authority` | `6pnj24Nk5M8PvntXqw4ZPqjXeQoBo7ZCxSUoVm8Cn1Ct` |

## Fee (konstanta program, permanen)

| Fee | BPS | Keterangan |
| :--- | :--- | :--- |
| Donation fee | 200 (2%) | donor → treasury |
| Claim fee | 500 (5%) | escrow → treasury |
| Sweep split | 70/20/10 | buyback / ops / bounty |
