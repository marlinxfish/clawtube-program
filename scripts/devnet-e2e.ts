// Devnet E2E smoke: real tip_sol against the deployed program.
// Proves: config PDA exists, whitelist check passes, escrow+entry created, fee split correct.
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { createHash } from "crypto";
import * as fs from "fs";

const RPC = "https://api.devnet.solana.com";
const PROGRAM_ID = new PublicKey("FvqRRnu1tfaJbUUGbnA3oaVuGUFHz8fLEc7Yr6rWE4aA");

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

const conn = new Connection(RPC, "confirmed");
const donor = load("keys/faucet.json"); // reuse faucet wallet as donor for the smoke test
const [configPda] = PublicKey.findProgramAddressSync([Buffer.from("config")], PROGRAM_ID);
const [treasuryPda] = PublicKey.findProgramAddressSync([Buffer.from("treasury")], PROGRAM_ID);

const platform = 0;
const uid = "UC_smoke_e2e_" + Date.now();
const hash = creatorHash(platform, uid);
const [escrowPda] = PublicKey.findProgramAddressSync([Buffer.from("escrow"), hash], PROGRAM_ID);

async function main() {
  const amount = 50_000_000n; // 0.05 SOL
  const data = Buffer.concat([
    disc("tip_sol"),
    Buffer.from([platform]),
    (() => { const b = Buffer.alloc(4 + uid.length); b.writeUInt32LE(uid.length, 0); b.write(uid, 4); return b; })(),
    (() => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(amount)); return b; })(),
  ]);

  const entryIdxBuf = Buffer.alloc(8);
  entryIdxBuf.writeBigUInt64LE(0n);
  const [entry] = PublicKey.findProgramAddressSync(
    [Buffer.from("entry"), hash, entryIdxBuf],
    PROGRAM_ID
  );

  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: configPda, isSigner: false, isWritable: false },
      { pubkey: treasuryPda, isSigner: false, isWritable: true },
      { pubkey: escrowPda, isSigner: false, isWritable: true },
      { pubkey: entry, isSigner: false, isWritable: true },
      { pubkey: donor.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
  const sig = await sendAndConfirmTransaction(conn, new Transaction().add(ix), [donor]);
  console.log("[OK] tip_sol:", sig);

  const escrow = await conn.getAccountInfo(escrowPda);
  const entryAcc = await conn.getAccountInfo(entry);
  const treasury = await conn.getAccountInfo(treasuryPda);
  if (!escrow || !entryAcc) throw new Error("escrow/entry account missing");

  // Entry layout: disc(8) creator_hash(16) index(8) amount(8) → amount at offset 32
  const entryAmount = entryAcc.data.readBigUInt64LE(32);
  const net = amount; // DONATION_FEE_BPS = 0 (100% net to creator escrow)
  if (entryAmount !== net) throw new Error(`entry amount ${entryAmount} != expected net ${net}`);
  console.log(`entry amount: ${Number(entryAmount) / LAMPORTS_PER_SOL} SOL (100% net, 0% fee) [OK]`);
  console.log("treasury PDA:", treasuryPda.toBase58());
  console.log("\nE2E PASS");
}

main().catch((e) => { console.error("E2E FAIL:", e.message ?? e); process.exit(1); });
