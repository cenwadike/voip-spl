import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Voip } from "../target/types/voip";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, Signer, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import assert from "assert";
import { BN } from "bn.js";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
const fs = require("fs");

const TestProgram = async() => {
  console.log("-------------------------------SET UP BEGIN-----------------------------------");
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Voip as Program<Voip>;

  const METADATA_SEED = "metadata";
  const TOKEN_METADATA_PROGRAM_ID = new PublicKey(
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
  );

  const MINT_SEED = "mint";
  const SETTINGS_SEED = "settings"
  // const payer = program.provider.publicKey;
  // const payer = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync("/Users/cenwadike/Dev/Playground/smart-contracts/solana/spl/payer-keypair.json"))));
  // const admin = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync("/Users/cenwadike/Dev/Playground/smart-contracts/solana/spl/mint-authority-keypair.json"))));
  const admin = Keypair.generate();
  const adminSig: Signer = {
    publicKey: admin.publicKey,
    secretKey: admin.secretKey
  }

  await program.provider.connection.confirmTransaction(
    await program.provider.connection.requestAirdrop(
      admin.publicKey,
      3 * LAMPORTS_PER_SOL
    ),
    "confirmed"
  );

  await program.provider.connection.confirmTransaction(
    await program.provider.connection.requestAirdrop(
      program.provider.publicKey,
      3 * LAMPORTS_PER_SOL
    ),
    "confirmed"
  );

  const metadata = {
    name: "VOIP",
    symbol: "VOP",
    uri: "https://arweave.net/Xjqaj_rYYQGrsiTk9JRqpguA813w6NGPikcRyA1vAHM",
    decimals: 9
  }
  const mintAmount = 100;
  const burnAmount = 10;

  const [mint] = PublicKey.findProgramAddressSync(
    [Buffer.from(MINT_SEED)],
    program.programId
  );

  const [settings] = PublicKey.findProgramAddressSync(
    [Buffer.from(SETTINGS_SEED)],
    program.programId
  );

  const [metadataAddress] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(METADATA_SEED),
      TOKEN_METADATA_PROGRAM_ID.toBuffer(),
      mint.toBuffer(),
    ],
    TOKEN_METADATA_PROGRAM_ID
  );

  console.log("-------------------------------SET UP COMPLETE-----------------------------------");
  console.log("-----------------------MINT ADDRESS: ", mint.toBase58());
  console.log("-----------------------PAYER ADDRESS: ", admin.publicKey.toBase58());
  console.log("-----------------------PROGRAM ID: ", program.programId.toBase58());

  console.log("-------------------------------INITIALIZATION BEGIN-----------------------------------");

  const info = await program.provider.connection.getAccountInfo(mint);
  if (!info) {
    console.log("  Mint not found. Initializing Program...");

    const initContext = {
      metadata: metadataAddress,
      mint,
      payer: admin.publicKey,
      settings: settings,
      rent: SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
    };

    const initTxHash = await program.methods
      .initialize(metadata)
      .accounts(initContext)
      .signers([adminSig])
      .rpc();

    await program.provider.connection.confirmTransaction(initTxHash, "finalized");
    console.log("Your transaction signature", initTxHash);

    const newInfo = await program.provider.connection.getAccountInfo(mint);
    assert(newInfo, "  Mint should be initialized.");

  } else {    
    // Do not attempt to initialize if already initialized
    console.log("  Mint already found.");
    console.log("  Mint: ", mint.toBase58());
    // return; 
  }
  
  console.log("-------------------------------INITIALIZATION COMPLETE-----------------------------------");

  console.log("-------------------------------MINT BEGIN-----------------------------------");
  const destination = anchor.utils.token.associatedAddress({
    mint: mint,
    owner: admin.publicKey,
  });
  let initialMintBalance: number;

  try {
    const balance = await program.provider.connection.getTokenAccountBalance(destination);
    initialMintBalance = balance.value.uiAmount;
  } catch {
    // Token account not yet initiated has 0 balance
    initialMintBalance = 0;
  }

  const context = {
    mint,
    destination,
    payer: admin.publicKey,
    rent: SYSVAR_RENT_PUBKEY,
    systemProgram: SystemProgram.programId,
    tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
    associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
  };

  const mintTxHash = await program.methods
    .mintTokens(new BN(mintAmount * 10 ** metadata.decimals))
    .accounts(context)
    .signers([adminSig])
    .rpc();

  await program.provider.connection.confirmTransaction(mintTxHash);
  console.log(`Mint token with transaction hash: ${mintTxHash}`);

  const postMintBalance = (
    await program.provider.connection.getTokenAccountBalance(destination)
  ).value.uiAmount;
  assert.equal(
    initialMintBalance + mintAmount,
    postMintBalance,
    "Compare balances, it must be equal"
  );
  console.log("-------------------------------MINT COMPLETE-----------------------------------");

  console.log("-------------------------------BURN BEGIN-----------------------------------");
  const from = anchor.utils.token.associatedAddress({
    mint: mint,
    owner: admin.publicKey,
  });

  const burnContext = {
    mint,
    from,
    payer: admin.publicKey,
    rent: SYSVAR_RENT_PUBKEY,
    systemProgram: SystemProgram.programId,
    tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
    associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
  };

  const burnTxHash = await program.methods
    .burnTokens(new BN(burnAmount * 10 ** metadata.decimals))
    .accounts(burnContext)
    .signers([adminSig])
    .rpc();

  await program.provider.connection.confirmTransaction(burnTxHash);
  console.log(`Burn token with transaction hash: ${burnTxHash}`);
  console.log("-------------------------------BURN COMPLETE-----------------------------------");

  console.log("-------------------------------SET TRADING BEGIN-----------------------------------");
    const setTradingContext = {
    settings,
    owner: admin.publicKey
  };

  const setTradingHash = await program.methods
    .setTrading(true)
    .accounts(setTradingContext)
    .signers([adminSig])
    .rpc();

  await program.provider.connection.confirmTransaction(setTradingHash);
  console.log(`Set trading successful with transaction hash: ${setTradingHash}`);
  console.log("-------------------------------SET TRADING COMPLETE-----------------------------------");

  console.log("-------------------------------TRANSFER BEGIN-----------------------------------");
  const transferAmmount = 10;
  
  const transferContext = {
    mint,
    from,
    to: from,
    authority: admin.publicKey,
    settings,
    tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
  };

  const transferHash = await program.methods
    .transferToken(new BN(transferAmmount * 10 ** metadata.decimals))
    .accounts(transferContext)
    .signers([adminSig])
    .rpc();

  await program.provider.connection.confirmTransaction(transferHash);
  console.log(`Transfer successful with transaction hash: ${transferHash}`);

  console.log("-------------------------------TRANSFER COMPLETE-----------------------------------");

  console.log("-------------------------------STUCK TOKEN BEGIN-----------------------------------");
  const to = anchor.utils.token.associatedAddress({
    mint: mint,
    owner: Keypair.generate().publicKey,
  });
  const stuckAmmount = 1;
  const SOL_MINT = new PublicKey("So11111111111111111111111111111111111111111");
  const stuckContext = {
    mint,
    stuckTokenMint: SOL_MINT,
    from: admin.publicKey, // change to contract account
    to: admin.publicKey,
    fromAta: from, // token from account ata
    toAta: to, // token to account ata
    payer: admin.publicKey,
    settings,
    tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId
  };

  const stuckHash = await program.methods
    .claimStuckTokens(new BN(stuckAmmount * 10 ** metadata.decimals))
    .accounts(stuckContext)
    .signers([adminSig])
    .rpc();

  await program.provider.connection.confirmTransaction(stuckHash);
  console.log(`Unstuck SOL with transaction hash: ${stuckHash}`);

  console.log("-------------------------------STUCK TOKEN COMPLETE-----------------------------------");
}

const runTest = async () => {
  try {
    await TestProgram();
    process.exit(0);
  } catch (error) {
    console.error(error);
    process.exit(1);
  }
}

runTest()