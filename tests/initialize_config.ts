import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { expect } from "chai";
import { Clawtube } from "../target/types/clawtube";
import { getConfigPda } from "./helpers";

describe("initialize_config", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Clawtube as Program<Clawtube>;

  it("initializes config with authority, registry bitmask, and null oracle", async () => {
    const authority = (provider.wallet as anchor.Wallet).payer;
    const [configPda, expectedBump] = getConfigPda(program.programId);

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
      // If already initialized in persistent ledger, proceed to assertions
    }

    const config = await program.account.platformConfig.fetch(configPda);
    expect(config.authority.toBase58()).to.equal(authority.publicKey.toBase58());
    expect(config.oracle).to.be.null;
    expect(config.platformRegistry).to.equal(0x01);
    expect(config.splMintCount).to.equal(0);
    expect(config.bump).to.equal(expectedBump);
  });
});
