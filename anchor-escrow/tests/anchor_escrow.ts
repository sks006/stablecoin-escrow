// Anchor TS client placeholder
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorEscrow } from "../target/types/anchor_escrow"; 
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { 
  TOKEN_2022_PROGRAM_ID, 
  createMint, 
  createAccount, 
  mintTo 
} from "@solana/spl-token";
import { expect } from "chai";

// 🔥 CU Reporter — fetches tx metadata and prints compute unit usage
async function logComputeUnits(
  connection: anchor.web3.Connection,
  txSig: string,
  label: string
): Promise<void> {
  // Confirm the transaction first so metadata is available
  const latestBlockhash = await connection.getLatestBlockhash();
  await connection.confirmTransaction({
    signature: txSig,
    ...latestBlockhash,
  }, "confirmed");

  const tx = await connection.getTransaction(txSig, {
    commitment: "confirmed",
    maxSupportedTransactionVersion: 0,
  });

  const cuUsed = tx?.meta?.computeUnitsConsumed ?? null;
  const cuLimit = 200_000; // default CU limit per instruction

  console.log(`\n${'═'.repeat(52)}`);
  console.log(`⚡ COMPUTE UNITS REPORT — ${label}`);
  console.log(`${'─'.repeat(52)}`);

  if (cuUsed !== null) {
    const pct = ((cuUsed / cuLimit) * 100).toFixed(1);
    const barLen = 30;
    const filled = Math.round((cuUsed / cuLimit) * barLen);
    const bar = '🟩'.repeat(Math.min(filled, barLen)) + '⬜'.repeat(Math.max(barLen - filled, 0));

    let emoji: string;
    if (cuUsed < 50_000) emoji = '🟢';
    else if (cuUsed < 100_000) emoji = '🟡';
    else if (cuUsed < 150_000) emoji = '🟠';
    else emoji = '🔴';

    console.log(`${emoji} CU Used:    ${cuUsed.toLocaleString()} / ${cuLimit.toLocaleString()}`);
    console.log(`📊 Usage:     ${pct}%`);
    console.log(`📈 Gauge:     ${bar}`);
    console.log(`💰 Fee:       ${(tx?.meta?.fee ?? 0).toLocaleString()} lamports`);
  } else {
    console.log(`❓ CU data unavailable for this transaction`);
  }

  console.log(`${'═'.repeat(52)}\n`);
}

describe("anchor-escrow-client-boundary", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AnchorEscrow as Program<AnchorEscrow>;

  let maker: Keypair;
  let taker: Keypair;
  let stablecoinMint: PublicKey;
  let makerTokenAccount: PublicKey;
  let escrowPda: PublicKey;
  let escrowVaultKeypair: Keypair;

  // 🛡️ SHIFT TO `before` - State must persist across the entire test suite
  before(async () => {
    maker = Keypair.generate();
    taker = Keypair.generate();

    // Fund maker
    const airdropSig = await provider.connection.requestAirdrop(maker.publicKey, 10_000_000_000);
    const latestBlockhash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      signature: airdropSig,
      ...latestBlockhash,
    });

    // Fund taker (Required to pay transaction fees in Phase 2)
    const takerAirdropSig = await provider.connection.requestAirdrop(taker.publicKey, 10_000_000_000);
    await provider.connection.confirmTransaction({
      signature: takerAirdropSig,
      ...latestBlockhash,
    });

    // 🛡️ STRICT TOKEN-2022 ALLOCATION
    stablecoinMint = await createMint(
      provider.connection,
      maker,
      maker.publicKey,
      null,
      6,
      Keypair.generate(),
      undefined,
      TOKEN_2022_PROGRAM_ID // Force Token-2022 compliance
    );

    makerTokenAccount = await createAccount(
      provider.connection,
      maker,
      stablecoinMint,
      maker.publicKey,
      Keypair.generate(),
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    await mintTo(
      provider.connection,
      maker,
      stablecoinMint,
      makerTokenAccount,
      maker,
      1_000_000_000,
      [],
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
  });

  it("Executes IDL-compliant Initialization", async () => {
    const amount = new anchor.BN(500_000_000); 
    const transferFeeBps = 25; 
    const allowlistRoot = Array.from({ length: 32 }, () => 0);
    const blacklistRoot = Array.from({ length: 32 }, () => 0);

    // Phase 1: Cryptographic Derivation — escrow PDA
    const [escrowPdaAddr] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), maker.publicKey.toBuffer(), taker.publicKey.toBuffer(), stablecoinMint.toBuffer()],
      program.programId
    );
    escrowPda = escrowPdaAddr;

    // Vault is an `init`-ed token account (no seeds), so we use a Keypair
    escrowVaultKeypair = Keypair.generate();

    // Phase 2 & 3: Instruction Packing + RPC Invocation
    // IDL arg order: amount, transfer_fee_bps, allowlist_merkle_root, blacklist_merkle_root, taker, cancel_authority
    const txSig = await program.methods
      .initializeEscrow(
        amount,
        transferFeeBps,
        allowlistRoot,
        blacklistRoot,
        taker.publicKey,
        maker.publicKey,  // cancel_authority = maker for this test
      )
      .accounts({
        maker: maker.publicKey,
        stablecoinMint,
        makerTokenAccount,
        escrowAccount: escrowPda,
        escrowVault: escrowVaultKeypair.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([maker, escrowVaultKeypair])
      .rpc();

    console.log("✅ Initialize tx:", txSig);
    await logComputeUnits(provider.connection, txSig, "initializeEscrow");

    // Phase 4: State Verification
    const escrowState = await program.account.anchorStablecoinEscrow.fetch(escrowPda);
    expect(escrowState.maker.toBase58()).to.equal(maker.publicKey.toBase58());
    expect(escrowState.amount.toNumber()).to.equal(amount.toNumber());
  });

  it("Executes IDL-compliant Transfer Settlement", async () => {
    // Create taker token account
    const takerTokenAccount = await createAccount(
      provider.connection,
      taker,
      stablecoinMint,
      taker.publicKey,
      Keypair.generate(),
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    const txSig = await program.methods
      .executeTransfer()
      .accounts({
        taker: taker.publicKey,
        takerTokenAccount,
        makerTokenAccount,
        escrowAccount: escrowPda,
        makerRef: maker.publicKey,
        escrowVault: escrowVaultKeypair.publicKey,
        stablecoinMint,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([taker])
      .rpc();

    console.log("✅ Transfer tx:", txSig);
    await logComputeUnits(provider.connection, txSig, "executeTransfer");

    // Phase 3: Garbage Collection Verification
    try {
      await program.account.anchorStablecoinEscrow.fetch(escrowPda);
      expect.fail("FATAL: Escrow account should have been closed");
    } catch (e) {
      expect((e as Error).toString()).to.include("Account does not exist");
      console.log("✅ Escrow PDA successfully reclaimed by VM");
    }
  });
});