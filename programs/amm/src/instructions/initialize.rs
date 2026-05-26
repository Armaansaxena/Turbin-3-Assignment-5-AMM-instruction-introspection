use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    constants::*,
    errors::AmmError,
    state::Pool,
};

/// Accounts required to initialize a new AMM pool.
/// Large accounts are Boxed to keep the stack frame under the 4096-byte SBF limit.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Token A mint
    pub mint_a: Box<Account<'info, Mint>>,

    /// Token B mint
    pub mint_b: Box<Account<'info, Mint>>,

    /// Pool state PDA — seeds: [POOL_SEED, mint_a, mint_b]
    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [POOL_SEED, mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// LP token mint — created by this instruction
    #[account(
        init,
        payer = payer,
        mint::decimals = LP_TOKEN_DECIMALS,
        mint::authority = pool,
        seeds = [LP_MINT_SEED, pool.key().as_ref()],
        bump,
    )]
    pub lp_mint: Box<Account<'info, Mint>>,

    /// Vault for token A — PDA-owned token account
    #[account(
        init,
        payer = payer,
        token::mint = mint_a,
        token::authority = pool,
        seeds = [VAULT_A_SEED, pool.key().as_ref()],
        bump,
    )]
    pub vault_a: Box<Account<'info, TokenAccount>>,

    /// Vault for token B — PDA-owned token account
    #[account(
        init,
        payer = payer,
        token::mint = mint_b,
        token::authority = pool,
        seeds = [VAULT_B_SEED, pool.key().as_ref()],
        bump,
    )]
    pub vault_b: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Handler: initialize a new constant-product AMM pool
pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
    require!(fee_bps <= MAX_FEE_BPS, AmmError::FeeTooHigh);

    // ── Enforce mint ordering for deterministic seeds ───────────────────
    // This ensures a unique PDA per token pair and prevents duplicate pools.
    require!(
        ctx.accounts.mint_a.key() < ctx.accounts.mint_b.key(),
        AmmError::InvalidMint
    );

    let pool = &mut ctx.accounts.pool;
    let bumps = &ctx.bumps;

    pool.authority    = ctx.accounts.payer.key();
    pool.mint_a       = ctx.accounts.mint_a.key();
    pool.mint_b       = ctx.accounts.mint_b.key();
    pool.vault_a      = ctx.accounts.vault_a.key();
    pool.vault_b      = ctx.accounts.vault_b.key();
    pool.lp_mint      = ctx.accounts.lp_mint.key();
    pool.fee_bps      = fee_bps;
    pool.reserve_a    = 0;
    pool.reserve_b    = 0;
    pool.lp_supply    = 0;
    pool.bump         = bumps.pool;
    pool.lp_mint_bump = bumps.lp_mint;
    pool.vault_a_bump = bumps.vault_a;
    pool.vault_b_bump = bumps.vault_b;

    msg!(
        "Pool initialized | mint_a={} mint_b={} fee_bps={}",
        pool.mint_a, pool.mint_b, pool.fee_bps
    );

    Ok(())
}
