use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use anchor_lang::solana_program::sysvar::instructions::{load_current_index_checked, load_instruction_at_checked};

use crate::{
    constants::*,
    errors::AmmError,
    state::Pool,
};

#[derive(Accounts)]
pub struct Payout<'info> {
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

    pub token_program: Program<'info, Token>,

    /// CHECK: Instructions sysvar for introspection
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn payout(
    ctx: Context<Payout>,
    min_amount_a: u64,
    min_amount_b: u64,
) -> Result<()> {
    require!(!ctx.accounts.pool.locked, AmmError::PoolLocked);
    // 1. Introspection: Find the BurnLp instruction
    let ixs = ctx.accounts.instructions.as_ref();
    let current_index = load_current_index_checked(ixs).map_err(|_| AmmError::InvalidInstructionIndex)? as usize;
    if current_index == 0 {
        return Err(AmmError::MissingBurnInstruction.into());
    }
    let prev_ix = load_instruction_at_checked(current_index - 1, ixs).map_err(|_| AmmError::MissingBurnInstruction)?;

    // 2. Verify it's our program and the BurnLp instruction
    // We compare program_id with current program's ID
    if prev_ix.program_id != *ctx.program_id {
        return Err(AmmError::InvalidBurnInstruction.into());
    }

    // Discriminator for "global:burn_lp"
    let expected_discriminator = [135, 136, 81, 255, 56, 231, 141, 85];
    if prev_ix.data.len() < 16 || prev_ix.data[0..8] != expected_discriminator {
        return Err(AmmError::InvalidBurnInstruction.into());
    }

    // 3. Extract lp_amount from BurnLp data (8 bytes discriminator + 8 bytes lp_amount)
    let lp_amount = u64::from_le_bytes(prev_ix.data[8..16].try_into().unwrap());

    // 4. Verify accounts in BurnLp match this Payout
    // BurnLp accounts: 0: user, 1: pool
    if prev_ix.accounts[0].pubkey != ctx.accounts.user.key() ||
       prev_ix.accounts[1].pubkey != ctx.accounts.pool.key() {
        return Err(AmmError::InvalidBurnInstruction.into());
    }

    // 5. Calculate payout share
    let pool = &ctx.accounts.pool;
    let lp_supply = pool.lp_supply;
    require!(lp_amount <= lp_supply, AmmError::MathOverflow);

    let amount_a = (lp_amount as u128)
        .checked_mul(pool.reserve_a as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::InsufficientLiquidity)? as u64;

    let amount_b = (lp_amount as u128)
        .checked_mul(pool.reserve_b as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::InsufficientLiquidity)? as u64;

    require!(amount_a >= min_amount_a, AmmError::SlippageExceeded);
    require!(amount_b >= min_amount_b, AmmError::SlippageExceeded);
    require!(amount_a > 0 && amount_b > 0, AmmError::InsufficientLiquidity);

    // 6. Transfer tokens from vaults to user
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

    // 7. Update pool state
    let pool = &mut ctx.accounts.pool;
    pool.reserve_a = pool.reserve_a.checked_sub(amount_a).ok_or(AmmError::MathOverflow)?;
    pool.reserve_b = pool.reserve_b.checked_sub(amount_b).ok_or(AmmError::MathOverflow)?;
    pool.lp_supply = pool.lp_supply.checked_sub(lp_amount).ok_or(AmmError::MathOverflow)?;

    msg!(
        "Payout | lp_burned={} amount_a={} amount_b={} reserve_a={} reserve_b={}",
        lp_amount, amount_a, amount_b, pool.reserve_a, pool.reserve_b
    );

    Ok(())
}
