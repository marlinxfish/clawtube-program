import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { Clawtube } from "../target/types/clawtube";
import { creatorHash, getConfigPda, getEscrowPda, getTreasuryPda } from "./helpers";

describe("claim", () => {
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

  it("fails when attestation is expired", async () => {
    const payout = Keypair.generate();
    const platform = 0; // YouTube
    const platformUid = "UC_claim_expired_test";
    const cHash = creatorHash(platform, platformUid);
    const [escrowPda] = getEscrowPda(cHash, program.programId);

    const pastTimestamp = new anchor.BN(Math.floor(Date.now() / 1000) - 3600); // 1 hour ago

    let failed = false;
    try {
      await program.methods
        .claim(
          platform,
          platformUid,
          "@creator",
          new anchor.BN(1),
          pastTimestamp,
          []
        )
        .accounts({
          config: configPda,
          treasury: treasuryPda,
          escrow: escrowPda,
          payout: payout.publicKey,
          sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([payout])
        .rpc();
    } catch (err: unknown) {
      failed = true;
      expect(String(err)).to.include("AttestationExpired");
    }
    expect(failed).to.be.true;
  });
});
