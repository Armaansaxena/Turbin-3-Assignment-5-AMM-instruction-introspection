use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;
pub mod errors;
pub mod constants;

use instructions::*;

declare_id!("2AixtNQtF9kMe3n4GmUAXnWSqkrfGZyEazexwytCbuw8");

#[program]
pub mod amm {
    use super::*;

    /// Initialize a new AMM pool with two token mints.
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        instructions::initialize::initialize(ctx, fee_bps)
    }

    /// Deposit tokens into the pool and receive LP tokens in return.
    pub fn deposit(
        ctx: Context<Deposit>,
        amount_a: u64,
        amount_b: u64,
        min_lp_tokens: u64,
    ) -> Result<()> {
        instructions::deposit::deposit(ctx, amount_a, amount_b, min_lp_tokens)
    }

    /// Step 1 of withdrawal: Burn LP tokens.
    /// Must be followed by `payout` in the same transaction.
    pub fn burn_lp(ctx: Context<BurnLp>, lp_amount: u64) -> Result<()> {
        instructions::burn_lp::burn_lp(ctx, lp_amount)
    }

    /// Step 2 of withdrawal: Verify burn and payout proportional share of reserves.
    /// Uses instruction introspection to verify `burn_lp` occurred in the same TX.
    pub fn payout(
        ctx: Context<Payout>,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        instructions::payout::payout(ctx, min_amount_a, min_amount_b)
    }

    /// Swap using constant product formula: x * y = k
    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<()> {
        instructions::swap::swap(ctx, amount_in, minimum_amount_out)
    }
}
