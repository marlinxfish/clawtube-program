// Devnet faucet: distribute SOL + whitelisted SPL tokens to testnet users.
// Usage: bun run scripts/devnet-faucet.ts <recipient> [amount_sol]
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createTransferInstruction,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { createHash } from "crypto";
import * as fs from "fs";

const RPC = "https://api.devnet.solana.com";
const PROGRAM_ID = new PublicKey("FvqRRnu1tfaJbUUGbnA3oaVuGUFHz8fLEc7Yr6rWE4aA");

const MINTS: Record<string, { mint: PublicKey; decimals: number }> = {
  CLAWTUBE: { mint: new PublicKey("3BDw2BVdpPxHf4uf2yLRgQbhMbnzEEsch9CRScu4otwg"), decimals: 6 },
  CLAW: { mint: new PublicKey("GEfEtWBGr8LpEc8PEyDtef4pYiewFjQ5e1srfbQTNSVm"), decimals: 6 },
  ANSEMHACK: { mint: new PublicKey("2TDdv5xYKQoPXN5JTs95DJRtxT3Hztf45BE9xdydfbJ6"), decimals: 6 },
};

const faucet = Keypair.fromSecretKey(
  Buffer.from(JSON.parse(fs.readFileSync("keys/faucet.json", "utf8")))
);
// Deployer holds all minted SPL supply
const deployer = Keypair.fromSecretKey(
  Buffer.from(JSON.parse(fs.readFileSync(process.env.HOME + "/.config/solana/keys/deployer.json", "utf8")))
);

const conn = new Connection(RPC, "confirmed");

function disc(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

function escrowPda(platform: number, uid: string): PublicKey {
  const hash = createHash("sha256")
    .update(Buffer.concat([Buffer.from([platform]), Buffer.from(uid)]))
    .digest()
    .subarray(0, 16);
  return PublicKey.findProgramAddressSync([Buffer.from("escrow"), hash], PROGRAM_ID)[0];
}

// Fund the escrow vault with wrapped SOL so tip_sol transfers clear.
// (tip_sol transfers raw SOL into the escrow PDA; vault must hold lamports.)
async function main() {
  const [recipientArg, solArg] = process.argv.slice(2);
  if (!recipientArg) {
    console.log("Usage: bun run scripts/devnet-faucet.ts <recipient> [amount_sol]");
    console.log("  recipient: wallet address | channel id (UC...) to fund escrow");
    process.exit(1);
  }

  const solAmount = solArg ? parseFloat(solArg) : 0.1;

  if (recipientArg.startsWith("UC") || recipientArg.startsWith("escrow:")) {
    const uid = recipientArg.replace("escrow:", "");
    const escrow = escrowPda(0, uid); // platform 0 = YouTube
    const lamports = Math.floor(solAmount * LAMPORTS_PER_SOL);
    const tx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: faucet.publicKey,
        toPubkey: escrow,
        lamports,
      })
    );
    const sig = await conn.sendTransaction(tx, [faucet]);
    console.log(`[OK] funded escrow ${escrow.toBase58()} with ${solAmount} SOL: ${sig}`);
    return;
  }

  const recipient = new PublicKey(recipientArg);
  const tx = new Transaction();

  // SOL from faucet wallet
  tx.add(
    SystemProgram.transfer({
      fromPubkey: faucet.publicKey,
      toPubkey: recipient,
      lamports: Math.floor(solAmount * LAMPORTS_PER_SOL),
    })
  );

  // SPL: ATA of deployer -> ATA of recipient (create recipient ATA if needed)
  for (const [symbol, { mint, decimals }] of Object.entries(MINTS)) {
    const srcAta = await getAssociatedTokenAddress(mint, deployer.publicKey);
    const dstAta = await getAssociatedTokenAddress(mint, recipient);
    const dstInfo = await conn.getAccountInfo(dstAta);
    if (!dstInfo) {
      tx.add(
        createAssociatedTokenAccountInstruction(faucet.publicKey, dstAta, recipient, mint)
      );
    }
    const srcBalance = await conn.getTokenAccountBalance(srcAta);
    const raw = BigInt(srcBalance.value.amount);
    const share = (raw * BigInt(1)) / BigInt(10); // drip 10% of remaining supply per call
    if (share > 0) {
      tx.add(
        createTransferInstruction(srcAta, dstAta, deployer.publicKey, share, [], TOKEN_PROGRAM_ID)
      );
      console.log(`  + ${symbol}: ${Number(share) / 10 ** decimals}`);
    } else {
      console.log(`  - ${symbol}: deployer balance empty, skipped`);
    }
  }

  const sig = await conn.sendTransaction(tx, [faucet, deployer]);
  console.log(`[OK] faucet -> ${recipient.toBase58()} (${solAmount} SOL): ${sig}`);
}

main().catch((e) => {
  console.error("Faucet failed:", e.message ?? e);
  process.exit(1);
});
