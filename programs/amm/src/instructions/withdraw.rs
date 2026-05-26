use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::*,
    errors::AmmError,
    state::Pool,
};

/// Accounts required to withdraw liquidity from the pool.
/// Large accounts are Boxed to keep the stack frame under the 4096-byte SBF limit.
#[derive(Accounts)]
pub struct Withdraw<'info> {
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
        constraint = user_lp_token.mint == pool.lp_mint @ AmmError::InvalidMint,
        constraint = user_lp_token.owner == user.key(),
    )]
    pub user_lp_token: Box<Account<'info, TokenAccount>>,

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

/// Handler: burn LP tokens and withdraw proportional share of pool reserves.
pub fn withdraw(
    ctx: Context<Withdraw>,
    lp_amount: u64,
    min_amount_a: u64,
    min_amount_b: u64,
) -> Result<()> {
    require!(lp_amount > 0, AmmError::ZeroAmount);

    let pool = &ctx.accounts.pool;
    require!(lp_amount <= pool.lp_supply, AmmError::InsufficientLpBalance);

    let amount_a = (lp_amount as u128)
        .checked_mul(pool.reserve_a as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(pool.lp_supply as u128)
        .ok_or(AmmError::InsufficientLiquidity)? as u64;

    let amount_b = (lp_amount as u128)
        .checked_mul(pool.reserve_b as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(pool.lp_supply as u128)
        .ok_or(AmmError::InsufficientLiquidity)? as u64;

    require!(amount_a >= min_amount_a, AmmError::SlippageExceeded);
    require!(amount_b >= min_amount_b, AmmError::SlippageExceeded);
    require!(amount_a > 0 && amount_b > 0, AmmError::InsufficientLiquidity);

    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint:      ctx.accounts.lp_mint.to_account_info(),
                from:      ctx.accounts.user_lp_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        lp_amount,
    )?;

    let mint_a_key = pool.mint_a;
    let mint_b_key = pool.mint_b;
    let bump = pool.bump;
    let pool_seeds: &[&[u8]] = &[POOL_SEED, mint_a_key.as_ref(), mint_b_key.as_ref(), &[bump]];
    let signer_seeds = &[pool_seeds];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.vault_a.to_account_info(),
                to:        ctx.accounts.user_token_a.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        ),
        amount_a,
    )?;

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.vault_b.to_account_info(),
                to:        ctx.accounts.user_token_b.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        ),
        amount_b,
    )?;

    let pool = &mut ctx.accounts.pool;
    pool.reserve_a = pool.reserve_a.checked_sub(amount_a).ok_or(AmmError::MathOverflow)?;
    pool.reserve_b = pool.reserve_b.checked_sub(amount_b).ok_or(AmmError::MathOverflow)?;
    pool.lp_supply = pool.lp_supply.checked_sub(lp_amount).ok_or(AmmError::MathOverflow)?;

    msg!(
        "Withdraw | lp_burned={} amount_a={} amount_b={} reserve_a={} reserve_b={}",
        lp_amount, amount_a, amount_b, pool.reserve_a, pool.reserve_b
    );

    Ok(())
}
