use anchor_lang::prelude::*;
use crate::state::Pool;
use crate::constants::POOL_SEED;

#[derive(Accounts)]
pub struct ToggleLock<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
        seeds = [POOL_SEED, pool.mint_a.as_ref(), pool.mint_b.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Box<Account<'info, Pool>>,
}

pub fn toggle_lock(ctx: Context<ToggleLock>, locked: bool) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    pool.locked = locked;

    msg!("Pool lock status updated | locked={}", locked);
    Ok(())
}
