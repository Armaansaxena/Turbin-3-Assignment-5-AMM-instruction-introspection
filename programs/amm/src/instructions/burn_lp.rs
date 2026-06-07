use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount};
use crate::{constants::*, errors::AmmError, state::Pool};

#[derive(Accounts)]
pub struct BurnLp<'info> {
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

    pub token_program: Program<'info, Token>,
}

pub fn burn_lp(ctx: Context<BurnLp>, lp_amount: u64) -> Result<()> {
    require!(lp_amount > 0, AmmError::ZeroAmount);
    
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

    msg!("BurnLp | lp_amount={}", lp_amount);
    Ok(())
}
