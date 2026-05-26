import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import { 
  TOKEN_PROGRAM_ID, 
  createMint, 
  createAccount, 
  mintTo, 
  getAccount, 
  getMint 
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

  before(async () => {
    // Create Mints
    mintA = await createMint(provider.connection, payer, payer.publicKey, null, 6);
    mintB = await createMint(provider.connection, payer, payer.publicKey, null, 6);

    // Sort mints for PDA derivation
    const [m0, m1] = mintA.toBuffer().compare(mintB.toBuffer()) < 0 
      ? [mintA, mintB] 
      : [mintB, mintA];

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

    // User Accounts
    userTokenA = await createAccount(provider.connection, payer, mintA, payer.publicKey);
    userTokenB = await createAccount(provider.connection, payer, mintB, payer.publicKey);
    userLpToken = await createAccount(provider.connection, payer, lpMintPda, payer.publicKey);

    // Mint tokens to user
    await mintTo(provider.connection, payer, mintA, userTokenA, payer, 10_000_000);
    await mintTo(provider.connection, payer, mintB, userTokenB, payer, 10_000_000);
  });

  it("Initializes the pool", async () => {
    const feeBps = 30; // 0.3%
    await program.methods
      .initialize(feeBps)
      .accounts({
        payer: payer.publicKey,
        mintA: mintA,
        mintB: mintB,
        // @ts-ignore - anchor-handled PDAs
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
        userTokenA: userTokenA,
        userTokenB: userTokenB,
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
    
    // sqrt(1e6 * 1e6) - 1000 = 999,000
    assert.equal(poolState.lpSupply.toNumber(), 999_000);
  });

  it("Swaps A for B", async () => {
    const amountIn = new anchor.BN(100_000);
    const minAmountOut = new anchor.BN(1);

    await program.methods
      .swap(amountIn, minAmountOut)
      .accounts({
        user: payer.publicKey,
        // @ts-ignore
        pool: poolPda,
        userSource: userTokenA,
        userDestination: userTokenB,
        // @ts-ignore
        vaultIn: vaultAPda,
        // @ts-ignore
        vaultOut: vaultBPda,
      })
      .rpc();

    const poolState = await program.account.pool.fetch(poolPda);
    // reserveA = 1,000,000 + 100,000 = 1,100,000
    assert.equal(poolState.reserveA.toNumber(), 1_100_000);
    // amount_in_after_fee = 100,000 * 0.997 = 99,700
    // amount_out = 1,000,000 * 99,700 / (1,000_000 + 99,700) = 90,661
    // reserveB = 1,000,000 - 90,661 = 909,339
    assert.approximately(poolState.reserveB.toNumber(), 909339, 1);
  });

  it("Withdraws liquidity", async () => {
    const lpAmount = new anchor.BN(100_000);
    const minA = new anchor.BN(1);
    const minB = new anchor.BN(1);

    await program.methods
      .withdraw(lpAmount, minA, minB)
      .accounts({
        user: payer.publicKey,
        // @ts-ignore
        pool: poolPda,
        // @ts-ignore
        lpMint: lpMintPda,
        userLpToken: userLpToken,
        userTokenA: userTokenA,
        userTokenB: userTokenB,
        // @ts-ignore
        vaultA: vaultAPda,
        // @ts-ignore
        vaultB: vaultBPda,
      })
      .rpc();

    const poolState = await program.account.pool.fetch(poolPda);
    // Previous supply: 999,000. Burned: 100,000. New: 899,000
    assert.equal(poolState.lpSupply.toNumber(), 899_000);
  });
});
