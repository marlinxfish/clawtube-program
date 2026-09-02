// Devnet setup: initialize_config + update_whitelist + set_oracle
// Manual discriminators (Anchor sha256("global:<name>")[..8]) — IDL not required.
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { createHash } from "crypto";
import * as fs from "fs";

const RPC = process.env.RPC_URL || "https://api.devnet.solana.com";

// Validate required parameters explicitly — NO SILENT FALLBACKS (||)
const rawProgramId = process.env.PROGRAM_ID;
if (!rawProgramId || !rawProgramId.trim()) {
  throw new Error("❌ Error: PROGRAM_ID is required! Please pass PROGRAM_ID=<PUBKEY>");
}
const PROGRAM_ID = new PublicKey(rawProgramId.trim());

const rawOracle = process.env.ORACLE_PUBKEY;
if (!rawOracle || !rawOracle.trim()) {
  throw new Error("❌ Error: ORACLE_PUBKEY is required! Please pass ORACLE_PUBKEY=<PUBKEY>");
}
const ORACLE = new PublicKey(rawOracle.trim());

const rawMintClawtube = process.env.MINT_CLAWTUBE;
const rawMintClaw = process.env.MINT_CLAW;
if (!rawMintClawtube || !rawMintClawtube.trim()) {
  throw new Error("❌ Error: MINT_CLAWTUBE is required! Please pass MINT_CLAWTUBE=<MINT_PUBKEY>");
}
if (!rawMintClaw || !rawMintClaw.trim()) {
  throw new Error("❌ Error: MINT_CLAW is required! Please pass MINT_CLAW=<MINT_PUBKEY>");
}

const MINTS = [
  new PublicKey(rawMintClawtube.trim()), // CLAWTUBE
  new PublicKey(rawMintClaw.trim()),     // CLAW
];

const deployerKeyPath = process.env.DEPLOYER_KEYPAIR_PATH || process.env.KEYPAIR_PATH;
if (!deployerKeyPath || !deployerKeyPath.trim() || !fs.existsSync(deployerKeyPath.trim())) {
  throw new Error(`❌ Error: DEPLOYER_KEYPAIR_PATH is required and must exist! Path: ${deployerKeyPath}`);
}

const deployer = Keypair.fromSecretKey(
  Buffer.from(JSON.parse(fs.readFileSync(deployerKeyPath.trim(), "utf8")))
);
const conn = new Connection(RPC, "confirmed");
function disc(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

const [configPda] = PublicKey.findProgramAddressSync([Buffer.from("config")], PROGRAM_ID);
const [treasuryPda] = PublicKey.findProgramAddressSync([Buffer.from("treasury")], PROGRAM_ID);

// Borsh-serialize: platform_registry u8, oracle Option<Pubkey>
function encodeInitializeConfig(platformRegistry: number, oracle: PublicKey | null): Buffer {
  const head = Buffer.concat([disc("initialize_config"), Buffer.from([platformRegistry])]);
  if (oracle === null) return Buffer.concat([head, Buffer.from([0])]);
  return Buffer.concat([head, Buffer.from([1]), oracle.toBuffer()]);
}

// Vec<Pubkey>: u32 len + entries
function encodeUpdateWhitelist(mints: PublicKey[]): Buffer {
  const body = Buffer.alloc(4 + 32 * mints.length);
  body.writeUInt32LE(mints.length, 0);
  mints.forEach((m, i) => m.toBuffer().copy(body, 4 + i * 32));
  return Buffer.concat([disc("update_whitelist"), body]);
}

// Option<Pubkey> = Some(oracle)
function encodeSetOracle(oracle: PublicKey): Buffer {
  return Buffer.concat([disc("set_oracle"), Buffer.from([1]), oracle.toBuffer()]);
}

async function send(ix: { keys: any[], data: Buffer }, label: string) {
  const tx = new Transaction().add({
    programId: PROGRAM_ID,
    keys: ix.keys,
    data: ix.data,
  });
  const sig = await sendAndConfirmTransaction(conn, tx, [deployer]);
  console.log(`[OK] ${label}: ${sig}`);
}

const isInitialized = await conn.getAccountInfo(configPda);
if (!isInitialized) {
  await send(
    {
      keys: [
        { pubkey: configPda, isSigner: false, isWritable: true },
        { pubkey: deployer.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: encodeInitializeConfig(0x01, ORACLE),
    },
    "initialize_config (registry=0x01, oracle=faucet)"
  );
} else {
  console.log("[SKIP] config already initialized");
}

await send(
  {
    keys: [
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: deployer.publicKey, isSigner: true, isWritable: false },
    ],
    data: encodeUpdateWhitelist(MINTS),
  },
  "update_whitelist (CLAWTUBE, CLAW)"
);

console.log("\nConfig PDA:", configPda.toBase58());
console.log("Treasury PDA:", treasuryPda.toBase58());
console.log("Oracle:", ORACLE.toBase58());
