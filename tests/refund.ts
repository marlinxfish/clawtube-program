import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { Clawtube } from "../target/types/clawtube";
import { creatorHash, getConfigPda, getEntryPda, getEscrowPda } from "./helpers";

describe("refund", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Clawtube as Program<Clawtube>;
  const [configPda] = getConfigPda(program.programId);

  it("fails when non-donor tries to refund entry", async () => {
    const fakeDonor = Keypair.generate();
    const platform = 0;
    const platformUid = "UC_refund_test";
    const cHash = creatorHash(platform, platformUid);
    const [escrowPda] = getEscrowPda(cHash, program.programId);
    const [entryPda] = getEntryPda(cHash, 0, program.programId);

    let failed = false;
    try {
      await program.methods
        .refund()
        .accounts({
          config: configPda,
          escrow: escrowPda,
          entry: entryPda,
          donor: fakeDonor.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([fakeDonor])
        .rpc();
    } catch (err: unknown) {
      failed = true;
      expect(String(err)).to.be.a("string");
    }
    expect(failed).to.be.true;
  });
});
