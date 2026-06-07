import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import { 
  TOKEN_PROGRAM_ID, 
  createMint, 
  createAccount, 
  mintTo, 
} from "@solana/spl-token";
import { assert } from "chai";

describe("amm", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Amm as Program<Amm>;
  const payer = (provider.wallet as anchor.Wallet).payer;

  let mintA: anchor.web3.PublicKey;
  let mintB: anchor.web3.PublicKey;
  let userTokenA: anchor.web3.PublicKey;
  let userTokenB: anchor.web3.PublicKey;
  let userLpToken: anchor.web3.PublicKey;

  let poolPda: anchor.web3.PublicKey;
  let lpMintPda: anchor.web3.PublicKey;
  let vaultAPda: anchor.web3.PublicKey;
  let vaultBPda: anchor.web3.PublicKey;

  let m0: anchor.web3.PublicKey;
  let m1: anchor.web3.PublicKey;
  let userToken0: anchor.web3.PublicKey;
  let userToken1: anchor.web3.PublicKey;

  before(async () => {
    // Create Mints
    mintA = await createMint(provider.connection, payer, payer.publicKey, null, 6, undefined, undefined, TOKEN_PROGRAM_ID);
    mintB = await createMint(provider.connection, payer, payer.publicKey, null, 6, undefined, undefined, TOKEN_PROGRAM_ID);

    // Sort mints for PDA derivation and program requirement
    if (mintA.toBuffer().compare(mintB.toBuffer()) < 0) {
      m0 = mintA;
      m1 = mintB;
    } else {
      m0 = mintB;
      m1 = mintA;
    }

    // Derivations
    [poolPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), m0.toBuffer(), m1.toBuffer()],
      program.programId
    );

    [lpMintPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lp_mint"), poolPda.toBuffer()],
      program.programId
    );

    [vaultAPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault_a"), poolPda.toBuffer()],
      program.programId
    );

    [vaultBPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault_b"), poolPda.toBuffer()],
      program.programId
    );

    // User Accounts for Mint A and B
    userTokenA = await createAccount(provider.connection, payer, mintA, payer.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);
    userTokenB = await createAccount(provider.connection, payer, mintB, payer.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);

    // Map user tokens to sorted mints
    if (mintA.toBuffer().compare(mintB.toBuffer()) < 0) {
      userToken0 = userTokenA;
      userToken1 = userTokenB;
    } else {
      userToken0 = userTokenB;
      userToken1 = userTokenA;
    }

    // Mint tokens to user
    await mintTo(provider.connection, payer, mintA, userTokenA, payer, 10_000_000, [], undefined, TOKEN_PROGRAM_ID);
    await mintTo(provider.connection, payer, mintB, userTokenB, payer, 10_000_000, [], undefined, TOKEN_PROGRAM_ID);
  });

  it("Initializes the pool", async () => {
    const feeBps = 30; // 0.3%
    await program.methods
      .initialize(feeBps)
      .accounts({
        payer: payer.publicKey,
        mintA: m0,
        mintB: m1,
        // @ts-ignore
        pool: poolPda,
        // @ts-ignore
        lpMint: lpMintPda,
        // @ts-ignore
        vaultA: vaultAPda,
        // @ts-ignore
        vaultB: vaultBPda,
      })
      .rpc();

    const poolState = await program.account.pool.fetch(poolPda);
    assert.equal(poolState.feeBps, feeBps);
    assert.equal(poolState.reserveA.toNumber(), 0);
    assert.equal(poolState.reserveB.toNumber(), 0);

    // Now that LP mint is initialized, create the user LP token account
    userLpToken = await createAccount(provider.connection, payer, lpMintPda, payer.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);
  });

  it("Deposits liquidity (First Deposit)", async () => {
    const amountA = new anchor.BN(1_000_000);
    const amountB = new anchor.BN(1_000_000);
    const minLp = new anchor.BN(100);

    await program.methods
      .deposit(amountA, amountB, minLp)
      .accounts({
        user: payer.publicKey,
        // @ts-ignore
        pool: poolPda,
        // @ts-ignore
        lpMint: lpMintPda,
        userTokenA: userToken0, // Mapped to m0
        userTokenB: userToken1, // Mapped to m1
        userLpToken: userLpToken,
        // @ts-ignore
        vaultA: vaultAPda,
        // @ts-ignore
        vaultB: vaultBPda,
      })
      .rpc();

    const poolState = await program.account.pool.fetch(poolPda);
    assert.equal(poolState.reserveA.toNumber(), 1_000_000);
    assert.equal(poolState.reserveB.toNumber(), 1_000_000);
    assert.equal(poolState.lpSupply.toNumber(), 999_000);
  });

  it("Swaps token0 for token1", async () => {
    const amountIn = new anchor.BN(100_000);
    const minAmountOut = new anchor.BN(1);

    await program.methods
      .swap(amountIn, minAmountOut)
      .accounts({
        user: payer.publicKey,
        // @ts-ignore
        pool: poolPda,
        userSource: userToken0,
        userDestination: userToken1,
        // @ts-ignore
        vaultIn: vaultAPda,
        // @ts-ignore
        vaultOut: vaultBPda,
      })
      .rpc();

    const poolState = await program.account.pool.fetch(poolPda);
    // reserveA = 1,000,000 + 100,000 = 1,100,000
    assert.equal(poolState.reserveA.toNumber(), 1_100_000);
    // reserveB approx 909,339
    assert.approximately(poolState.reserveB.toNumber(), 909339, 1);
  });

  it("Withdraws liquidity (split instructions with introspection)", async () => {
    const lpAmount = new anchor.BN(100_000);
    const minA = new anchor.BN(1);
    const minB = new anchor.BN(1);

    const tx = new anchor.web3.Transaction();

    tx.add(
      await program.methods
        .burnLp(lpAmount)
        .accounts({
          user: payer.publicKey,
          // @ts-ignore
          pool: poolPda,
          // @ts-ignore
          lpMint: lpMintPda,
          userLpToken: userLpToken,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .instruction()
    );

    tx.add(
      await program.methods
        .payout(minA, minB)
        .accounts({
          user: payer.publicKey,
          // @ts-ignore
          pool: poolPda,
          userTokenA: userToken0,
          userTokenB: userToken1,
          // @ts-ignore
          vaultA: vaultAPda,
          // @ts-ignore
          vaultB: vaultBPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          instructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        })
        .instruction()
    );

    await provider.sendAndConfirm(tx);

    const poolState = await program.account.pool.fetch(poolPda);
    // Previous supply: 999,000. Burned: 100,000. New: 899,000
    assert.equal(poolState.lpSupply.toNumber(), 899_000);
  });

  it("Fails to payout without burn instruction", async () => {
    const minA = new anchor.BN(1);
    const minB = new anchor.BN(1);

    try {
      await program.methods
        .payout(minA, minB)
        .accounts({
          user: payer.publicKey,
          // @ts-ignore
          pool: poolPda,
          userTokenA: userToken0,
          userTokenB: userToken1,
          // @ts-ignore
          vaultA: vaultAPda,
          // @ts-ignore
          vaultB: vaultBPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          instructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        })
        .rpc();
      assert.fail("Should have failed");
    } catch (e: any) {
      // Expected: MissingBurnInstruction
      assert.include(e.message, "MissingBurnInstruction");
    }
  });

  it("Fails to payout if burn instruction is in wrong order", async () => {
    const lpAmount = new anchor.BN(100_000);
    const minA = new anchor.BN(1);
    const minB = new anchor.BN(1);

    const tx = new anchor.web3.Transaction();

    // Payout FIRST (will be index 0)
    tx.add(
      await program.methods
        .payout(minA, minB)
        .accounts({
          user: payer.publicKey,
          // @ts-ignore
          pool: poolPda,
          userTokenA: userToken0,
          userTokenB: userToken1,
          // @ts-ignore
          vaultA: vaultAPda,
          // @ts-ignore
          vaultB: vaultBPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          instructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        })
        .instruction()
    );

    // Burn SECOND
    tx.add(
      await program.methods
        .burnLp(lpAmount)
        .accounts({
          user: payer.publicKey,
          // @ts-ignore
          pool: poolPda,
          // @ts-ignore
          lpMint: lpMintPda,
          userLpToken: userLpToken,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .instruction()
    );

    try {
      await provider.sendAndConfirm(tx);
      assert.fail("Should have failed");
    } catch (e: any) {
      // Expected: MissingBurnInstruction (since current_index is 0)
      assert.include(e.message, "MissingBurnInstruction");
    }
  });
});
