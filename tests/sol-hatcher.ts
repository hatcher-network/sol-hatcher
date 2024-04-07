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
  mintTo,
} from "@solana/spl-token";
import { testWallet } from "./lib/vars";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { Signer } from "@solana/web3.js";

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

  let winnerTokenAccountPubkey: PublicKey = undefined;
  let vaultTokenAccountPubkey: PublicKey = undefined;

  it("Prepare for test", async () => {
    // requestAirdrop for test wallet
    console.log("Request airdrop to custom wallet");
    const txSig = await connection.requestAirdrop(testWallet.publicKey, 10000000000000);
    console.log("Airdrop finished. Tx: ", txSig);
  })

  it("Initialize Token Mint", async () => {
    // PDA for the token metadata account for the reward token mint
    const hatcherTokenMintMetadataPDA = await metaplex
      .nfts()
      .pdas()
      .metadata({ mint: hatcherTokenMintPDA })

    const [vaultSigner] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vaultSigner")],
      program.programId
    )

    const tx = await program.methods
      .initializeData()
      .accounts({
        hatcherTokenMint: hatcherTokenMintPDA,
        metadataAccount: hatcherTokenMintMetadataPDA,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        hatchData: hatchData, // init
        vaultSigner: vaultSigner,
      })
      .rpc()

    console.log("Your transaction signature", tx)

    // create token account with vault signer
    vaultTokenAccountPubkey = await getOrCreateAssociatedTokenAccount(
      connection,
      payer.payer,
      hatcherTokenMintPDA,
      // program.programId
      vaultSigner, // not on curve
      true,
    ).then(res => res.address);

    console.log("vaultTokenAccount: ", vaultTokenAccountPubkey.toString());

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

  })

  it("Check balance and transfer: test_wallet to payer, 500", async () => {
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


  it("Deposit token: test_wallet, 10000 ", async () => {

    const testTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      testWallet,
      hatcherTokenMintPDA,
      testWallet.publicKey,
    )

    const [testTokenBalanceInVault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("userBalance"), testWallet.publicKey.toBuffer()],
      program.programId
    )

    console.log("All ATA Generated")

    let tx = await program.methods.depositToken(new BN(10000)).accounts({
      hatchData: hatchData,
      vaultTokenAccount: vaultTokenAccountPubkey,
      // user: payer.payer.publicKey,
      user: testWallet.publicKey,
      userTokenAccount: testTokenAccount.address,
      userBalanceAccount: testTokenBalanceInVault,
    }).signers([testWallet]).rpc();

    console.log("Transfer txSig: ", tx);

    let userBalanceData = await program.account.userBalance.fetch(testTokenBalanceInVault);
    console.log("Token Balance in vault - After deposit", userBalanceData);

    const _balance = await connection.getTokenAccountBalance(testTokenAccount.address).then((res) => res.value);

    console.log("User Balance - After Deposit", _balance.uiAmount);

  })

  it("Withdraw token", async () => {

    const testTokenAccountPubkey = await getAssociatedTokenAddressSync(
      hatcherTokenMintPDA,
      testWallet.publicKey
    )

    const [testTokenBalanceInVault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("userBalance"), testWallet.publicKey.toBuffer()],
      program.programId
    )

    const [vaultSigner] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vaultSigner")],
      program.programId
    )

    const tx = await program.methods.withdrawToken(new BN(5000)).accounts({
      hatchData: hatchData,
      vaultTokenAccount: vaultTokenAccountPubkey,
      user: testWallet.publicKey,
      userTokenAccount: testTokenAccountPubkey,
      userBalanceAccount: testTokenBalanceInVault,
      // vaultSigner: hatcherTokenVaultATA,
      vaultSigner: vaultSigner, // program.programId
    }).signers([testWallet]).rpc();
    // .transaction().serialize({requireAllSignatures: false, verifySignatures: false})

    console.log("Transfer txSig: ", tx);

    const userBalanceData = await program.account.userBalance.fetch(testTokenBalanceInVault);
    console.log("Token Balance in vault after withdraw", userBalanceData);

  })

  it("Mint to", async () => {

    const testTokenAccount = await getAssociatedTokenAddressSync(
      hatcherTokenMintPDA,
      testWallet.publicKey
    )

    let _balance = await connection.getTokenAccountBalance(testTokenAccount).then((res) => res.value);
    console.log("Balance Before Mint:", _balance.uiAmount);

    const [vaultSigner] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vaultSigner")],
      program.programId
    )

    const tx = await program.methods.mintToken(new BN(70000000))
      .accounts({
        userTokenAccount: winnerTokenAccountPubkey,
        user: testWallet.publicKey,
        hatcherTokenMint: hatcherTokenMintPDA,
      }).rpc()//

    console.log("Mint to tx:", tx);

    _balance = await connection.getTokenAccountBalance(testTokenAccount).then((res) => res.value);

    console.log("Balance After Mint:", _balance.uiAmount);

  })

});
