// Devnet security e2e: verifies the two critical fixes.
// 1. claim WITHOUT a valid oracle Ed25519 attestation is rejected (no more byte-window bypass).
// 2. SPL tip -> SPL claim actually transfers tokens out of the vault ATA.
//
// Usage: bun run scripts/devnet-security-e2e.ts
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  SYSVAR_INSTRUCTIONS_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import * as nacl from "tweetnacl";
import { createHash } from "crypto";
import * as fs from "fs";

const RPC = "https://api.devnet.solana.com";
const PROGRAM_ID = new PublicKey("FvqRRnu1tfaJbUUGbnA3oaVuGUFHz8fLEc7Yr6rWE4aA");
const ED25519_PROGRAM_ID = new PublicKey("Ed25519SigVerify111111111111111111111111111");
const AToken_PROGRAM_ID = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const CLAWTUBE_MINT = new PublicKey("3BDw2BVdpPxHf4uf2yLRgQbhMbnzEEsch9CRScu4otwg");

function disc(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}
function creatorHash(platform: number, uid: string): Buffer {
  return createHash("sha256")
    .update(Buffer.concat([Buffer.from([platform]), Buffer.from(uid)]))
    .digest()
    .subarray(0, 16);
}
function load(path: string): Keypair {
  return Keypair.fromSecretKey(Buffer.from(JSON.parse(fs.readFileSync(path, "utf8"))));
}
function borshString(s: string): Buffer {
  const b = Buffer.alloc(4 + s.length);
  b.writeUInt32LE(s.length, 0);
  b.write(s, 4);
  return b;
}

const conn = new Connection(RPC, "confirmed");
const oracle = load("keys/faucet.json"); // devnet oracle
const deployer = load(process.env.HOME + "/.config/solana/keys/deployer.json");

const [configPda] = PublicKey.findProgramAddressSync([Buffer.from("config")], PROGRAM_ID);
const [treasuryPda] = PublicKey.findProgramAddressSync([Buffer.from("treasury")], PROGRAM_ID);

function buildEd25519Ix(pubkey: PublicKey, message: Buffer, signature: Buffer): TransactionInstruction {
  const header = Buffer.alloc(16);
  header[0] = 1; // num_signatures
  header.writeUInt16LE(16, 2); // signatures_offset
  header.writeUInt16LE(0xffff, 4);
  header.writeUInt16LE(80, 6); // public_key_offset
  header.writeUInt16LE(0xffff, 8);
  header.writeUInt16LE(112, 10); // message_offset
  header.writeUInt16LE(message.length, 12); // message_size
  header.writeUInt16LE(0xffff, 14);
  const data = Buffer.concat([header, signature, pubkey.toBuffer(), message]);
  return new TransactionInstruction({ keys: [], programId: ED25519_PROGRAM_ID, data });
}

function attestationMsg(platform: number, uid: string, payout: PublicKey, nonce: bigint, expiresAt: bigint, handle: string): Buffer {
  const parts = [
    Buffer.from([1, platform]), // version + platform
    borshString(uid),
    payout.toBuffer(),
    (() => { const b = Buffer.alloc(8); b.writeBigUInt64LE(nonce); return b; })(),
    (() => { const b = Buffer.alloc(8); b.writeBigInt64LE(expiresAt); return b; })(),
    borshString(handle),
  ];
  return Buffer.concat(parts);
}

async function tipSol(donor: Keypair, platform: number, uid: string, amountLamports: number) {
  const hash = creatorHash(platform, uid);
  const [escrow] = PublicKey.findProgramAddressSync([Buffer.from("escrow"), hash], PROGRAM_ID);
  const [entry] = PublicKey.findProgramAddressSync([Buffer.from("entry"), hash, Buffer.alloc(8)], PROGRAM_ID);
  const data = Buffer.concat([
    disc("tip_sol"),
    Buffer.from([platform]),
    borshString(uid),
    (() => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(amountLamports)); return b; })(),
    Buffer.from(hash), // creator_hash [u8;16]
  ]);
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: configPda, isSigner: false, isWritable: false },
      { pubkey: treasuryPda, isSigner: false, isWritable: true },
      { pubkey: escrow, isSigner: false, isWritable: true },
      { pubkey: entry, isSigner: false, isWritable: true },
      { pubkey: donor.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
  await sendAndConfirmTransaction(conn, new Transaction().add(ix), [donor]);
  return { escrow, entry };
}

async function tipSpl(donor: Keypair, platform: number, uid: string, amountRaw: bigint) {
  const hash = creatorHash(platform, uid);
  const [escrow] = PublicKey.findProgramAddressSync([Buffer.from("escrow"), hash], PROGRAM_ID);
  const [entry] = PublicKey.findProgramAddressSync([Buffer.from("entry"), hash, Buffer.alloc(8)], PROGRAM_ID);
  const vault = await getAssociatedTokenAddress(CLAWTUBE_MINT, escrow, true);
  const donorAta = await getAssociatedTokenAddress(CLAWTUBE_MINT, donor.publicKey);
  const treasuryAta = await getAssociatedTokenAddress(CLAWTUBE_MINT, treasuryPda, true);
  const data = Buffer.concat([
    disc("tip_spl"),
    Buffer.from([platform]),
    borshString(uid),
    (() => { const b = Buffer.alloc(8); b.writeBigUInt64LE(amountRaw); return b; })(),
    Buffer.from(hash), // creator_hash [u8;16]
  ]);
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: configPda, isSigner: false, isWritable: false },
      { pubkey: treasuryPda, isSigner: false, isWritable: true },
      { pubkey: escrow, isSigner: false, isWritable: true },
      { pubkey: entry, isSigner: false, isWritable: true },
      { pubkey: CLAWTUBE_MINT, isSigner: false, isWritable: false },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: donorAta, isSigner: false, isWritable: true },
      { pubkey: treasuryAta, isSigner: false, isWritable: true },
      { pubkey: donor.publicKey, isSigner: true, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: AToken_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
  await sendAndConfirmTransaction(conn, new Transaction().add(ix), [donor]);
  return { escrow, entry, vault };
}
function claimData(platform: number, uid: string, handle: string, nonce: bigint, expiresAt: bigint, indices: bigint[], mint: PublicKey | null): Buffer {
  const idxBuf = Buffer.alloc(4 + 8 * indices.length);
  idxBuf.writeUInt32LE(indices.length, 0);
  indices.forEach((v, i) => idxBuf.writeBigUInt64LE(v, 4 + 8 * i));
  const mintBuf = mint === null ? Buffer.from([0]) : Buffer.concat([Buffer.from([1]), mint.toBuffer()]);
  const hash = creatorHash(platform, uid);
  return Buffer.concat([
    disc("claim"),
    Buffer.from([platform]),
    borshString(uid),
    borshString(handle),
    (() => { const b = Buffer.alloc(8); b.writeBigUInt64LE(nonce); return b; })(),
    (() => { const b = Buffer.alloc(8); b.writeBigInt64LE(expiresAt); return b; })(),
    idxBuf,
    mintBuf,
    Buffer.from(hash), // creator_hash [u8;16]
  ]);
}

async function main() {
  const platform = 0;
  const uid = "UC_security_" + Date.now();
  const payout = Keypair.generate();
  const donor = deployer;
  const nonce = 1n;
  const expiresAt = BigInt(Math.floor(Date.now() / 1000) + 3600);

  // ---- Setup: SOL escrow with one tip ----
  const { escrow } = await tipSol(donor, platform, uid, 50_000_000);
  console.log("[OK] tip_sol -> escrow", escrow.toBase58());
  const hash = creatorHash(platform, uid);
  const [entryPda] = PublicKey.findProgramAddressSync([Buffer.from("entry"), hash, Buffer.alloc(8)], PROGRAM_ID);

  // ---- Test 1: claim WITHOUT attestation must FAIL ----
  const claimIx1 = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: configPda, isSigner: false, isWritable: false },
      { pubkey: treasuryPda, isSigner: false, isWritable: true },
      { pubkey: escrow, isSigner: false, isWritable: true },
      { pubkey: payout.publicKey, isSigner: true, isWritable: true },
      { pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      // remaining: entry PDA
      { pubkey: entryPda, isSigner: false, isWritable: true },
    ],
    data: claimData(platform, uid, "@creator", nonce, expiresAt, [0n], null),
  });

  let rejected = false;
  try {
    await sendAndConfirmTransaction(conn, new Transaction().add(claimIx1), [donor, payout]);
  } catch (e) {
    rejected = true;
    const msg = String((e as Error)?.message ?? e);
    console.log("  reject reason:", msg.slice(0, 100));
  }
  console.log(rejected ? "[OK] claim without attestation REJECTED (oracle enforced)" : "[FAIL] claim without attestation SUCCEEDED?!");
  if (!rejected) process.exit(1);

  // ---- Test 2: SPL tip -> SPL claim transfers tokens ----
  const splUid = uid + "_spl";
  const tipAmount = 1_000_000n; // 1 CLAWTUBE
  const { vault: vaultAta, escrow: splEscrow } = await tipSpl(donor, platform, splUid, tipAmount);
  console.log("[OK] tip_spl -> vault", vaultAta.toBase58());

  const splHash = creatorHash(platform, splUid);
  const [splEntryPda] = PublicKey.findProgramAddressSync([Buffer.from("entry"), splHash, Buffer.alloc(8)], PROGRAM_ID);
  const splPayout = Keypair.generate();
  const payoutAta = await getOrCreateAssociatedTokenAccount(conn, donor, CLAWTUBE_MINT, splPayout.publicKey);
  const treasuryAta = await getAssociatedTokenAddress(CLAWTUBE_MINT, treasuryPda, true);

  const vaultBefore = await conn.getTokenAccountBalance(vaultAta);
  const payoutBefore = await conn.getTokenAccountBalance(payoutAta.address);

  // Valid attestation: oracle (faucet) signs the exact message.
  const msg = attestationMsg(platform, splUid, splPayout.publicKey, nonce, expiresAt, "@creator");
  const sig = Buffer.from(nacl.sign.detached(msg, oracle.secretKey));
  const edIx = buildEd25519Ix(oracle.publicKey, msg, sig);

  const splClaimIx = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: configPda, isSigner: false, isWritable: false },
      { pubkey: treasuryPda, isSigner: false, isWritable: true },
      { pubkey: splEscrow, isSigner: false, isWritable: true },
      { pubkey: splPayout.publicKey, isSigner: true, isWritable: true },
      { pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      // remaining: entry PDA + vault + payout ATA + treasury ATA
      { pubkey: splEntryPda, isSigner: false, isWritable: true },
      { pubkey: vaultAta, isSigner: false, isWritable: true },
      { pubkey: payoutAta.address, isSigner: false, isWritable: true },
      { pubkey: treasuryAta, isSigner: false, isWritable: true },
    ],
    data: claimData(platform, splUid, "@creator", nonce, expiresAt, [0n], CLAWTUBE_MINT),
  });

  const tx = new Transaction().add(edIx).add(splClaimIx);
  const sig2 = await sendAndConfirmTransaction(conn, tx, [donor, splPayout]);
  console.log("[OK] SPL claim tx:", sig2);

  const vaultAfter = await conn.getTokenAccountBalance(vaultAta);
  const payoutAfter = await conn.getTokenAccountBalance(payoutAta.address);
  const delta = BigInt(payoutAfter.value.amount) - BigInt(payoutBefore.value.amount);
  console.log(`vault  ${vaultBefore.value.amount} -> ${vaultAfter.value.amount}`);
  console.log(`payout ${payoutBefore.value.amount} -> ${payoutAfter.value.amount} (delta ${delta})`);

  if (delta <= 0n) {
    console.log("[FAIL] SPL tokens did NOT move to payout (still stranded)");
    process.exit(1);
  }
  console.log("\nSECURITY E2E PASS: oracle enforced, SPL claim transfers tokens");
}

main().catch((e) => { console.error("E2E FAIL:", (e as Error)?.message ?? e); process.exit(1); });
