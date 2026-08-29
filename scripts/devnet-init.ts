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

const RPC = "https://api.devnet.solana.com";
const PROGRAM_ID = new PublicKey("FvqRRnu1tfaJbUUGbnA3oaVuGUFHz8fLEc7Yr6rWE4aA");

// From token.md — devnet SPL whitelist
const MINTS = [
  new PublicKey("3BDw2BVdpPxHf4uf2yLRgQbhMbnzEEsch9CRScu4otwg"), // CLAWTUBE
  new PublicKey("GEfEtWBGr8LpEc8PEyDtef4pYiewFjQ5e1srfbQTNSVm"), // CLAW
  new PublicKey("2TDdv5xYKQoPXN5JTs95DJRtxT3Hztf45BE9xdydfbJ6"), // ANSEMHACK
];
const ORACLE = new PublicKey("AbvVdJsfagQPAMhSR8tcHYRbQmTLyrNaoNLvVF6pNTDr"); // faucet.json = devnet oracle

const deployer = Keypair.fromSecretKey(
  Buffer.from(JSON.parse(fs.readFileSync(process.env.HOME + "/.config/solana/keys/deployer.json", "utf8")))
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
  "update_whitelist (CLAWTUBE, CLAW, ANSEMHACK)"
);

console.log("\nConfig PDA:", configPda.toBase58());
console.log("Treasury PDA:", treasuryPda.toBase58());
console.log("Oracle:", ORACLE.toBase58());
