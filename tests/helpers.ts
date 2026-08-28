import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { createHash } from "crypto";
import { Clawtube } from "../target/types/clawtube";

export function getProgram(): Program<Clawtube> {
  return anchor.workspace.Clawtube as Program<Clawtube>;
}

export function creatorHash(platform: number, platformUid: string): Buffer {
  const hasher = createHash("sha256");
  hasher.update(Buffer.from([platform]));
  hasher.update(Buffer.from(platformUid, "utf8"));
  return hasher.digest().subarray(0, 16);
}

export function getConfigPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("config")], programId);
}

export function getTreasuryPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("treasury")], programId);
}

export function getEscrowPda(creatorHashBytes: Buffer, programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("escrow"), creatorHashBytes], programId);
}

export function getEntryPda(creatorHashBytes: Buffer, index: number | bigint, programId: PublicKey): [PublicKey, number] {
  const indexBuf = Buffer.alloc(8);
  indexBuf.writeBigUInt64LE(BigInt(index));
  return PublicKey.findProgramAddressSync([Buffer.from("entry"), creatorHashBytes, indexBuf], programId);
}

export async function initConfig(
  program: Program<Clawtube>,
  authority: anchor.web3.Keypair,
  platformRegistry: number = 0x01,
  oracle: PublicKey | null = null
): Promise<PublicKey> {
  const [configPda] = getConfigPda(program.programId);
  await program.methods
    .initializeConfig(platformRegistry, oracle)
    .accounts({
      config: configPda,
      authority: authority.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .signers([authority])
    .rpc();
  return configPda;
}
