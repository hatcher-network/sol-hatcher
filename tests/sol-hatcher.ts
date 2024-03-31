import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolHatcher } from "../target/types/sol_hatcher";
import { BN } from "bn.js";

describe("sol-hatcher", () => {
  // Configure the client to use the local cluster.

  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolHatcher as Program<SolHatcher>;
  // const program = anchor.workspace.SolanaHatcher as Program<SolHatcher>;
  // const wallet = anchor.workspace.SolHatcher.provider.wallet;
  console.log("wallet.publicKey", provider.publicKey);
  const [hatchData] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("hatchData"), provider.publicKey.toBuffer()],
    program.programId
  )
  console.log("test...");

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });

  it("update leaderboard", async () => {
    const tx = await program.methods.updateLeaderboard([{
      agentId: new BN(6666),
      creator: provider.publicKey,
      // creator: new publicKey("4wvkHZTw9HiV23zko2FogZAU5sjErwE34dKMSz2x1P93"),
      score: new BN(6666)
    }]).accounts({
        hatchData: hatchData
    }).rpc();
    console.log(tx);
    const data = await program.account.hatchData.fetch(hatchData);
    console.log("data", data)
  })
});
