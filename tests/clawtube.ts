import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Clawtube } from "../target/types/clawtube";

describe("clawtube", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Clawtube as Program<Clawtube>;

  it("Initializes", async () => {
    // Suite penuh (tip → claim → refund → sweep per platform) menyusul via implementation plan.
    // Lihat spec: docs/superpowers/specs/2026-08-28-multi-platform-design.md §11
  });
});
