import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { Clawtube } from "../target/types/clawtube";
import { creatorHash, getConfigPda, getEntryPda, getEscrowPda, getTreasuryPda } from "./helpers";

describe("tip_sol", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Clawtube as Program<Clawtube>;
  const authority = (provider.wallet as anchor.Wallet).payer;
  const [configPda] = getConfigPda(program.programId);
  const [treasuryPda] = getTreasuryPda(program.programId);

  before(async () => {
    try {
      await program.methods
        .initializeConfig(0x01, null)
        .accounts({
          config: configPda,
          authority: authority.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([authority])
        .rpc();
    } catch {
      // Already initialized
    }
  });

  it("fails when tipping 0 amount", async () => {
    const donor = (provider.wallet as anchor.Wallet).payer;
    const platform = 0; // YouTube
    const platformUid = "UC_test_zero_amount";
    const cHash = creatorHash(platform, platformUid);
    const [escrowPda] = getEscrowPda(cHash, program.programId);
    const [entryPda] = getEntryPda(cHash, 0, program.programId);

    let failed = false;
    try {
      await program.methods
        .tipSol(platform, platformUid, new anchor.BN(0))
        .accounts({
          config: configPda,
          treasury: treasuryPda,
          escrow: escrowPda,
          entry: entryPda,
          donor: donor.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([donor])
        .rpc();
    } catch (err: unknown) {
      failed = true;
      expect(String(err)).to.include("ZeroAmount");
    }
    expect(failed).to.be.true;
  });

  it("fails when platform is disabled in registry", async () => {
    const donor = (provider.wallet as anchor.Wallet).payer;
    const platform = 1; // X (disabled in 0x01 bitmask)
    const platformUid = "x_handle_disabled";
    const cHash = creatorHash(platform, platformUid);
    const [escrowPda] = getEscrowPda(cHash, program.programId);
    const [entryPda] = getEntryPda(cHash, 0, program.programId);

    let failed = false;
    try {
      await program.methods
        .tipSol(platform, platformUid, new anchor.BN(100_000_000))
        .accounts({
          config: configPda,
          treasury: treasuryPda,
          escrow: escrowPda,
          entry: entryPda,
          donor: donor.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([donor])
        .rpc();
    } catch (err: unknown) {
      failed = true;
      expect(String(err)).to.include("PlatformDisabled");
    }
    expect(failed).to.be.true;
  });

  it("fails when platform is invalid enum value", async () => {
    const donor = (provider.wallet as anchor.Wallet).payer;
    const platform = 99; // Invalid platform
    const platformUid = "invalid_platform_uid";
    const cHash = creatorHash(platform, platformUid);
    const [escrowPda] = getEscrowPda(cHash, program.programId);
    const [entryPda] = getEntryPda(cHash, 0, program.programId);

    let failed = false;
    try {
      await program.methods
        .tipSol(platform, platformUid, new anchor.BN(100_000_000))
        .accounts({
          config: configPda,
          treasury: treasuryPda,
          escrow: escrowPda,
          entry: entryPda,
          donor: donor.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([donor])
        .rpc();
    } catch (err: unknown) {
      failed = true;
      expect(String(err)).to.include("InvalidPlatform");
    }
    expect(failed).to.be.true;
  });
});
