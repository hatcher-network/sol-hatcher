import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolHatcher } from "../target/types/sol_hatcher";
import { BN } from "bn.js";
import { Metaplex, PublicKey } from "@metaplex-foundation/js";
import { PROGRAM_ID as TOKEN_METADATA_PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata"
import {
  getOrCreateAssociatedTokenAccount,
  getAssociatedTokenAddressSync,
  transfer,
} from "@solana/spl-token";
import { testWallet } from "./lib/vars";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

describe("sol-hatcher", () => {
  // Configure the client to use the local cluster.

  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolHatcher as Program<SolHatcher>;
  const connection = program.provider.connection

  console.log("Read program Id: ", program.programId)

  // const program = anchor.workspace.SolanaHatcher as Program<SolHatcher>;
  // const testWallet = anchor.workspace.SolHatcher.provider.wallet;

  const payer = anchor.workspace.SolHatcher.provider.wallet as NodeWallet;
  // console.log("payer: ", payer.payer.secretKey);

  const [hatchData] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("hatchData"), provider.publicKey.toBuffer()],
    program.programId
  )

  // console.log("payer secretKey: ", payer.secretKey);

  const metaplex = Metaplex.make(connection);

  const [hatcherTokenMintPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("hatcherToken")],
    program.programId
  )

  const space = 0;

  let winnerTokenAccountPubkey: PublicKey = undefined;

  it("Initialize Token Mint", async () => {
    // PDA for the token metadata account for the reward token mint
    const hatcherTokenMintMetadataPDA = await metaplex
      .nfts()
      .pdas()
      .metadata({ mint: hatcherTokenMintPDA })

    const tx = await program.methods
      .initializeData()
      .accounts({
        hatcherTokenMint: hatcherTokenMintPDA,
        metadataAccount: hatcherTokenMintMetadataPDA,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        hatchData: hatchData, // init
      })
      .rpc()

    console.log("Your transaction signature", tx)
  })

  it("update leaderboard", async () => {

    // create a creator account

    winnerTokenAccountPubkey = getAssociatedTokenAddressSync(
      hatcherTokenMintPDA,
      testWallet.publicKey
    )

    console.log("winnerTokenAccountPubkey", winnerTokenAccountPubkey)

    const tx = await program.methods.updateLeaderboard([{
      agentId: new BN(8123481234),
      creator: testWallet.publicKey,
      score: new BN(99)
    }]).accounts({
      hatchData: hatchData,
      // winnerTokenAccount: testWallet.publicKey,
      winnerTokenAccount: winnerTokenAccountPubkey,
      winnerAccount: testWallet.publicKey,
      hatcherTokenMint: hatcherTokenMintPDA,
    }).rpc();

    console.log(tx);

    const data = await program.account.hatchData.fetch(hatchData);
    console.log("data", data);

    // const wallet_balance = await 
  })

  it("Check balance and transfer", async () => {
    let _balance = await connection.getTokenAccountBalance(winnerTokenAccountPubkey).then((res) => res.value);
    console.log("User Balance - Before Transfer", _balance.uiAmount);

    // generate ATA for destination account
    const myTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      payer.payer,
      hatcherTokenMintPDA, // token mint
      payer.payer.publicKey, // owner, test Wallet
    ).then((res) => res.address);

    console.log("Payer ATA", myTokenAccount);
    console.log("testWallet Pubkey", testWallet.publicKey);

    const txSig = await transfer(
      connection,
      payer.payer, // payer
      winnerTokenAccountPubkey, // from
      myTokenAccount, // to 
      testWallet, // owner 
      500,
      [testWallet, payer.payer]
    );

    console.log("Transfer txSig: ", txSig);

    _balance = await connection.getTokenAccountBalance(winnerTokenAccountPubkey).then((res) => res.value);

    console.log("User Balance - After Transfer", _balance.uiAmount);

  })
});
