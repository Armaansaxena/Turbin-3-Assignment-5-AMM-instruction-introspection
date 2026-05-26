use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;
pub mod errors;
pub mod constants;

// Glob re-export brings Initialize, Deposit, Withdraw, Swap into scope
// No collision: each instruction file now has a uniquely-named public function
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

    /// Burn LP tokens and withdraw proportional share of pool reserves.
    pub fn withdraw(
        ctx: Context<Withdraw>,
        lp_amount: u64,
        min_amount_a: u64,
        min_amount_b: u64,
    ) -> Result<()> {
        instructions::withdraw::withdraw(ctx, lp_amount, min_amount_a, min_amount_b)
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
