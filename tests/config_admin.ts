import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { Clawtube } from "../target/types/clawtube";
import { getConfigPda } from "./helpers";

describe("config_admin", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Clawtube as Program<Clawtube>;
  const authority = (provider.wallet as anchor.Wallet).payer;
  const [configPda] = getConfigPda(program.programId);

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

  it("fails to update whitelist with more than 3 mints", async () => {
    const mints = [
      Keypair.generate().publicKey,
      Keypair.generate().publicKey,
      Keypair.generate().publicKey,
      Keypair.generate().publicKey,
    ];

    let failed = false;
    try {
      await program.methods
        .updateWhitelist(mints)
        .accounts({
          config: configPda,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc();
    } catch (err: unknown) {
      failed = true;
      const errorMsg = String(err);
      expect(errorMsg).to.include("WhitelistOverflow");
    }
    expect(failed).to.be.true;
  });

  it("updates whitelist with 2 mints successfully", async () => {
    const mint1 = Keypair.generate().publicKey;
    const mint2 = Keypair.generate().publicKey;

    await program.methods
      .updateWhitelist([mint1, mint2])
      .accounts({
        config: configPda,
        authority: authority.publicKey,
      })
      .signers([authority])
      .rpc();

    const config = await program.account.platformConfig.fetch(configPda);
    expect(config.splMintCount).to.equal(2);
    expect(config.splMints[0].toBase58()).to.equal(mint1.toBase58());
    expect(config.splMints[1].toBase58()).to.equal(mint2.toBase58());
  });

  it("sets oracle successfully", async () => {
    const oracleKey = Keypair.generate().publicKey;

    await program.methods
      .setOracle(oracleKey)
      .accounts({
        config: configPda,
        authority: authority.publicKey,
      })
      .signers([authority])
      .rpc();

    const config = await program.account.platformConfig.fetch(configPda);
    expect(config.oracle?.toBase58()).to.equal(oracleKey.toBase58());
  });

  it("fails when non-authority tries to update authority", async () => {
    const intruder = Keypair.generate();

    let failed = false;
    try {
      await program.methods
        .updateAuthority(intruder.publicKey)
        .accounts({
          config: configPda,
          authority: intruder.publicKey,
        })
        .signers([intruder])
        .rpc();
    } catch (err: unknown) {
      failed = true;
      const errorMsg = String(err);
      expect(errorMsg).to.include("ConstraintHasOne");
    }
    expect(failed).to.be.true;
  });
});
