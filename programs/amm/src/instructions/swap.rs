use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{
    constants::*,
    errors::AmmError,
    state::Pool,
};

/// Accounts required to execute a swap
#[derive(Accounts)]
pub struct Swap<'info> {
    /// The trader
    #[account(mut)]
    pub user: Signer<'info>,

    /// Pool state
    #[account(
        mut,
        seeds = [POOL_SEED, pool.mint_a.as_ref(), pool.mint_b.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// User's source token account (the token being sold)
    #[account(
        mut,
        constraint = (
            user_source.mint == pool.mint_a || user_source.mint == pool.mint_b
        ) @ AmmError::InvalidMint,
        constraint = user_source.owner == user.key(),
    )]
    pub user_source: Box<Account<'info, TokenAccount>>,

    /// User's destination token account (the token being received)
    #[account(
        mut,
        constraint = (
            user_destination.mint == pool.mint_a || user_destination.mint == pool.mint_b
        ) @ AmmError::InvalidMint,
        constraint = user_destination.owner == user.key(),
        constraint = user_destination.mint != user_source.mint @ AmmError::InvalidMint,
    )]
    pub user_destination: Box<Account<'info, TokenAccount>>,

    /// Pool vault for the input token
    #[account(
        mut,
        constraint = (
            vault_in.key() == pool.vault_a || vault_in.key() == pool.vault_b
        ) @ AmmError::InvalidVault,
    )]
    pub vault_in: Box<Account<'info, TokenAccount>>,

    /// Pool vault for the output token
    #[account(
        mut,
        constraint = (
            vault_out.key() == pool.vault_a || vault_out.key() == pool.vault_b
        ) @ AmmError::InvalidVault,
        constraint = vault_out.key() != vault_in.key() @ AmmError::InvalidVault,
    )]
    pub vault_out: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

/// Handler: constant-product swap (x * y = k) with fee deduction
///
/// Direction is inferred from the mint of `user_source`:
/// - If source mint == mint_a → selling A, buying B
/// - If source mint == mint_b → selling B, buying A
///
/// # Arguments
/// * `amount_in`         — exact amount of input token to sell
/// * `minimum_amount_out` — minimum output tokens to receive (slippage guard)
pub fn swap(
    ctx: Context<Swap>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<()> {
    require!(amount_in > 0, AmmError::ZeroAmount);

    let pool = &ctx.accounts.pool;

    // ── Determine swap direction ─────────────────────────────────────────
    let is_a_to_b = ctx.accounts.user_source.mint == pool.mint_a;

    let (reserve_in, reserve_out) = if is_a_to_b {
        (pool.reserve_a, pool.reserve_b)
    } else {
        (pool.reserve_b, pool.reserve_a)
    };

    require!(reserve_in > 0 && reserve_out > 0, AmmError::InsufficientLiquidity);

    // ── Apply fee: amount_in_after_fee = amount_in * (BPS - fee_bps) / BPS ─
    let fee_bps = pool.fee_bps as u64;
    let amount_in_after_fee = (amount_in as u128)
        .checked_mul((BPS_DENOMINATOR - fee_bps) as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(AmmError::MathOverflow)? as u64;

    // ── Constant product formula: dy = y * dx / (x + dx) ────────────────
    // amount_out = reserve_out * amount_in_after_fee / (reserve_in + amount_in_after_fee)
    let amount_out = (reserve_out as u128)
        .checked_mul(amount_in_after_fee as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(
            (reserve_in as u128)
                .checked_add(amount_in_after_fee as u128)
                .ok_or(AmmError::MathOverflow)?
        )
        .ok_or(AmmError::MathOverflow)? as u64;

    require!(amount_out > 0, AmmError::InsufficientLiquidity);
    require!(amount_out >= minimum_amount_out, AmmError::SlippageExceeded);

    let pool_seeds: &[&[u8]] = &[
        POOL_SEED,
        pool.mint_a.as_ref(),
        pool.mint_b.as_ref(),
        &[pool.bump],
    ];
    let signer_seeds = &[pool_seeds];

    // ── Transfer input tokens from user → vault_in ───────────────────────
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.user_source.to_account_info(),
                to:        ctx.accounts.vault_in.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_in,
    )?;

    // ── Transfer output tokens from vault_out → user ─────────────────────
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.vault_out.to_account_info(),
                to:        ctx.accounts.user_destination.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        ),
        amount_out,
    )?;

    // ── Update pool reserves ─────────────────────────────────────────────
    let pool = &mut ctx.accounts.pool;
    if is_a_to_b {
        pool.reserve_a = pool.reserve_a
            .checked_add(amount_in)
            .ok_or(AmmError::MathOverflow)?;
        pool.reserve_b = pool.reserve_b
            .checked_sub(amount_out)
            .ok_or(AmmError::MathOverflow)?;
    } else {
        pool.reserve_b = pool.reserve_b
            .checked_add(amount_in)
            .ok_or(AmmError::MathOverflow)?;
        pool.reserve_a = pool.reserve_a
            .checked_sub(amount_out)
            .ok_or(AmmError::MathOverflow)?;
    }

    msg!(
        "Swap | a_to_b={} amount_in={} amount_out={} fee_bps={} reserve_a={} reserve_b={}",
        is_a_to_b, amount_in, amount_out, fee_bps,
        pool.reserve_a, pool.reserve_b
    );

    Ok(())
}
