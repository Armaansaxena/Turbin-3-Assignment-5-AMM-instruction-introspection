use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::{
    constants::*,
    errors::AmmError,
    state::Pool,
};

/// Accounts required to deposit liquidity into the pool.
/// Large accounts are Boxed to keep the stack frame under the 4096-byte SBF limit.
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [POOL_SEED, pool.mint_a.as_ref(), pool.mint_b.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        address = pool.lp_mint,
        seeds = [LP_MINT_SEED, pool.key().as_ref()],
        bump = pool.lp_mint_bump,
    )]
    pub lp_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        constraint = user_token_a.mint == pool.mint_a @ AmmError::InvalidMint,
        constraint = user_token_a.owner == user.key(),
    )]
    pub user_token_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = user_token_b.mint == pool.mint_b @ AmmError::InvalidMint,
        constraint = user_token_b.owner == user.key(),
    )]
    pub user_token_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = user_lp_token.mint == pool.lp_mint @ AmmError::InvalidMint,
        constraint = user_lp_token.owner == user.key(),
    )]
    pub user_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        address = pool.vault_a,
        seeds = [VAULT_A_SEED, pool.key().as_ref()],
        bump = pool.vault_a_bump,
    )]
    pub vault_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        address = pool.vault_b,
        seeds = [VAULT_B_SEED, pool.key().as_ref()],
        bump = pool.vault_b_bump,
    )]
    pub vault_b: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

/// Handler: deposit tokens and mint LP tokens (geometric mean on first deposit,
/// proportional on subsequent deposits).
pub fn deposit(
    ctx: Context<Deposit>,
    amount_a: u64,
    amount_b: u64,
    min_lp_tokens: u64,
) -> Result<()> {
    require!(!ctx.accounts.pool.locked, AmmError::PoolLocked);
    require!(amount_a > 0 && amount_b > 0, AmmError::ZeroAmount);

    let pool = &ctx.accounts.pool;

    // ── Enforce deposit ratio (if pool is not empty) ─────────────────────
    if pool.lp_supply > 0 {
        // (amount_a / reserve_a) == (amount_b / reserve_b)
        // amount_a * reserve_b == amount_b * reserve_a
        let left_side = (amount_a as u128)
            .checked_mul(pool.reserve_b as u128)
            .ok_or(AmmError::MathOverflow)?;
        let right_side = (amount_b as u128)
            .checked_mul(pool.reserve_a as u128)
            .ok_or(AmmError::MathOverflow)?;

        // Allow for tiny rounding error (1 unit)
        let diff = if left_side > right_side {
            left_side - right_side
        } else {
            right_side - left_side
        };

        // We require exact or very close ratio to prevent accidental donation
        require!(diff <= (pool.reserve_a as u128).max(pool.reserve_b as u128), AmmError::InvalidDepositRatio);
    }

    let lp_tokens_to_mint: u64 = if pool.lp_supply == 0 {
        let geometric_mean = (amount_a as u128)
            .checked_mul(amount_b as u128)
            .ok_or(AmmError::MathOverflow)?;
        let sqrt = integer_sqrt(geometric_mean);
        sqrt.checked_sub(MINIMUM_LIQUIDITY as u128)
            .ok_or(AmmError::InsufficientLiquidity)? as u64
    } else {
        let lp_a = (amount_a as u128)
            .checked_mul(pool.lp_supply as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(pool.reserve_a as u128)
            .ok_or(AmmError::InsufficientLiquidity)?;
        let lp_b = (amount_b as u128)
            .checked_mul(pool.lp_supply as u128)
            .ok_or(AmmError::MathOverflow)?
            .checked_div(pool.reserve_b as u128)
            .ok_or(AmmError::InsufficientLiquidity)?;
        lp_a.min(lp_b) as u64
    };

    require!(lp_tokens_to_mint >= min_lp_tokens, AmmError::InsufficientLpTokens);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.user_token_a.to_account_info(),
                to:        ctx.accounts.vault_a.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_a,
    )?;

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.user_token_b.to_account_info(),
                to:        ctx.accounts.vault_b.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_b,
    )?;

    let mint_a_key = pool.mint_a;
    let mint_b_key = pool.mint_b;
    let bump = pool.bump;
    let pool_seeds: &[&[u8]] = &[POOL_SEED, mint_a_key.as_ref(), mint_b_key.as_ref(), &[bump]];
    let signer_seeds = &[pool_seeds];

    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint:      ctx.accounts.lp_mint.to_account_info(),
                to:        ctx.accounts.user_lp_token.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        ),
        lp_tokens_to_mint,
    )?;

    let pool = &mut ctx.accounts.pool;
    pool.reserve_a = pool.reserve_a.checked_add(amount_a).ok_or(AmmError::MathOverflow)?;
    pool.reserve_b = pool.reserve_b.checked_add(amount_b).ok_or(AmmError::MathOverflow)?;
    pool.lp_supply = pool.lp_supply.checked_add(lp_tokens_to_mint).ok_or(AmmError::MathOverflow)?;

    msg!(
        "Deposit | amount_a={} amount_b={} lp_minted={} reserve_a={} reserve_b={}",
        amount_a, amount_b, lp_tokens_to_mint, pool.reserve_a, pool.reserve_b
    );

    Ok(())
}

fn integer_sqrt(n: u128) -> u128 {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x { x = y; y = (x + n / x) / 2; }
    x
}
