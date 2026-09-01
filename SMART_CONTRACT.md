# ClawTube Smart Contract — Alur & Ekonomi Protokol

Dokumen ini menjelaskan cara kerja smart contract ClawTube secara detail: struktur akun on-chain, 9 instruksi, **alur fee dari sisi donatur, kreator, dan sistem**, serta siklus hidup dana. Nilai konstanta diambil langsung dari kode (`programs/clawtube/src/state.rs`).

---

## 1. Struktur Akun On-Chain (PDA)

| PDA | Seed | Isi |
| :--- | :--- | :--- |
| `ProtocolConfig` | `["config"]` | authority, oracle, platform_registry, whitelist 3 mint |
| `Treasury` | `["treasury"]` | fee sink SOL (akumulasi fee) |
| `CreatorEscrow` | `["escrow", creator_hash]` | brankas per-kreator: hash, uid, entry_count, nonce, vault_totals |
| `Entry` | `["entry", creator_hash, index]` | satu donasi: amount, mint, donor, status, expires_at, sweep_at |
| `Vault ATA` | ATA(escrow, mint) | saldo token SPL per-kreator (dibuat lazy saat SPL pertama) |
| `Buyback` | `["buyback"]` | tujuan sweep 70% (buyback & burn) |
| `Ops` | `["ops"]` | tujuan sweep 28% (infrastruktur & operasional) |

`creator_hash = sha256(platform ++ platform_uid)[:16]` — mengikat brankas ke identitas kreator secara kriptografis, sehingga tidak mungkin tertukar antar-kreator.

---

## 2. Sembilan Instruksi

| # | Instruksi | Fungsi | Pemanggil |
| :--- | :--- | :--- | :--- |
| 1 | `initialize_config` | Inisialisasi config PDA (authority, oracle, registry) | authority |
| 2 | `update_authority` | Ganti authority | authority |
| 3 | `set_oracle` | Ganti oracle (attestasi klaim) | authority |
| 4 | `update_whitelist` | Set hingga 3 mint SPL (tanpa redeploy) | authority |
| 5 | `tip_sol` | Tip native SOL | donatur |
| 6 | `tip_spl` | Tip token SPL whitelisted | donatur |
| 7 | `claim` | Klaim tip (dual-signature oracle + kreator) | kreator (payout) |
| 8 | `refund` | Refund tip kedaluwarsa | donatur pemilik entri |
| 9 | `sweep` | Sapu tip kedaluwarsa (>120 hari) | siapa pun (bounty) |

---

## 3. Alur Fee — Per Sisi

### 3.1 Sisi Donatur (Saat Tip)

```
Donasi X
 ├── 2%  (200 bps) → Treasury        ← fee donasi
 └── 98% (net)     → Escrow kreator  ← dana dititip
```

- Donatur menanggung **2% fee donasi** (dipotong on-chain dari jumlah yang dikirim).
- **Biaya tambahan (bukan fee protokol)**: rent untuk membuat akun `Entry` (~0.0014 SOL, dikembalikan penuh saat entri ditutup), plus rent escrow PDA + ATA vault pada **donasi pertama** ke kanal baru, plus network fee validator.
- `tip_sol` memakai lamports langsung; `tip_spl` memindahkan token dari ATA donatur ke vault ATA escrow.

### 3.2 Sisi Kreator (Saat Klaim)

```
Escrow (98% dari X)
 ├── 5%  (500 bps) → Treasury     ← fee klaim
 └── 95%           → Wallet kreator
```

- Kreator menanggung **5% fee klaim** atas jumlah yang diklaim (98% dari donasi awal).
- Klaim SPL pertama kali sekaligus membuat ATA payout kreator — sewa dipotong langsung dari hasil klaim (kreator berdompet kosong pun tetap bisa cair).
- Klaim butuh **dua tanda tangan**: attestasi Oracle (bukti kepemilikan channel) + tanda tangan wallet payout kreator.

### 3.3 Sisi Sistem (Treasury)

Treasury mengumpulkan **dua lapis fee**:

| Sumber | Besaran |
| :--- | :--- |
| Fee donasi | 2% dari setiap tip |
| Fee klaim | 5% dari setiap klaim |

**Total take maksimum = 6.9%** dari nilai donasi (permanen, konstanta program — tidak bisa diubah tanpa upgrade yang terlihat publik).

### 3.4 Refund Donatur (Hari 91–120) — Tanpa Fee

- Donatur menarik **100%** nominalnya (98% dari donasi awal) tanpa fee klaim.
- Hanya donatur pemilik entri (diverifikasi via tanda tangan) yang bisa refund.

### 3.5 Sweep (>120 Hari) — Split 70/28/2

```
Dana tersapu
 ├── 70% → Buyback Treasury    (buyback & burn $CLAWTUBE)
 ├── 28% → Ops Treasury        (infrastruktur & operasional)
 └──  2% → Sweeper (bounty)    (gas fee refund + insentif)
```

---

## 4. Contoh Angka (Donasi 1.000 SOL)

| Tahap | Nilai | Penerima |
| :--- | :--- | :--- |
| Donatur kirim | 1.000 SOL | — |
| Fee donasi (2%) | 0.020 SOL | Treasury |
| Masuk escrow (98%) | 0.980 SOL | Escrow kreator |
| Fee klaim (5% dari 0.980) | 0.049 SOL | Treasury |
| Kreator terima (95%) | **0.931 SOL** | Kreator |

**Dari 1.000 SOL: kreator 93.1%, treasury 6.9%.**

(Untuk token SPL 6 desimal, persentasenya identik — bedanya satuan raw units, bukan lamports.)

---

## 5. Siklus Hidup Dana (120 Hari)

```
Hari 0        → Tip masuk escrow, status Entry = Active
Hari 0–90     → Jendela klaim kreator (exclusive)
Hari 91–120   → Jendela refund donatur (klaim kreator tertutup)
Hari 121+     → Sweep permissionless ke treasury (split 70/28/2)
```

- `entry_count` naik monoton per donasi; setiap donasi = satu `Entry` PDA baru.
- Status entri: `Active` → `Claimed` / `Refunded` / `Swept` (transisi sekali, anti double-spend).
- Invarian on-chain: `saldo vault ≥ Σ entri aktif` ditegakkan dengan checked math pada setiap mutasi.

---

## 6. Keamanan On-Chain

- **Non-custodial** — program tidak pernah memegang private key; semua tanda tangan di klien.
- **Attestasi Ed25519 sungguhan** — klaim memverifikasi instruksi `Ed25519SigVerify` on-chain (pubkey == oracle, message == attestasi), bukan pencarian substring. Lihat `AUDIT.md` (CRIT-1).
- **Replay guard** — `claim_nonce` naik monoton; attestasi dibatasi 5 menit (`expires_at`).
- **Dual-signature** — oracle saja tidak cukup menguras escrow; wajib co-sign wallet kreator.
- **Checked math** — seluruh aritmatika anti overflow/underflow (`overflow-checks = true`).
- **Rent-exempt guard** — saldo minimum sewa selalu dipertahankan agar akun PDA tidak tertutup.
- **Whitelist on-chain** — mint SPL dibatasi ≤3 dan bisa diubah via `update_whitelist` tanpa redeploy program.

---

## 7. Alamat & Detail Terkait

- Alamat on-chain: [`information.md`](information.md)
- Laporan audit: [`AUDIT.md`](AUDIT.md)
- Spesifikasi penuh: [`../project.md`](../project.md)
