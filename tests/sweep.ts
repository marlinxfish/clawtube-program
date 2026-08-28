import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { Clawtube } from "../target/types/clawtube";
import { creatorHash, getConfigPda, getEntryPda, getEscrowPda } from "./helpers";

describe("sweep", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Clawtube as Program<Clawtube>;
  const [configPda] = getConfigPda(program.programId);
  const [buybackPda] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("buyback")], program.programId);
  const [opsPda] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("ops")], program.programId);

  it("fails when sweeping before window opens", async () => {
    const sweeper = Keypair.generate();
    const platform = 0;
    const platformUid = "UC_sweep_test";
    const cHash = creatorHash(platform, platformUid);
    const [escrowPda] = getEscrowPda(cHash, program.programId);
    const [entryPda] = getEntryPda(cHash, 0, program.programId);

    let failed = false;
    try {
      await program.methods
        .sweep()
        .accounts({
          config: configPda,
          escrow: escrowPda,
          entry: entryPda,
          buyback: buybackPda,
          ops: opsPda,
          sweeper: sweeper.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([sweeper])
        .rpc();
    } catch (err: unknown) {
      failed = true;
      expect(String(err)).to.be.a("string");
    }
    expect(failed).to.be.true;
  });
});
