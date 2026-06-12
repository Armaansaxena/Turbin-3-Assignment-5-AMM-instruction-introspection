use anchor_lang::prelude::*;

/// Core AMM pool state stored on-chain.
/// Holds references to vaults, LP mint, fee config, and reserve tracking.
#[account]
#[derive(Default)]
pub struct Pool {
    /// The authority (creator) of this pool
    pub authority: Pubkey,

    /// Mint of token A
    pub mint_a: Pubkey,

    /// Mint of token B
    pub mint_b: Pubkey,

    /// Vault holding token A reserves (PDA-owned token account)
    pub vault_a: Pubkey,

    /// Vault holding token B reserves (PDA-owned token account)
    pub vault_b: Pubkey,

    /// LP token mint — minted on deposit, burned on withdraw
    pub lp_mint: Pubkey,

    /// Fee in basis points (e.g. 30 = 0.30%)
    pub fee_bps: u16,

    /// Cached reserve of token A (kept in sync with vault)
    pub reserve_a: u64,

    /// Cached reserve of token B (kept in sync with vault)
    pub reserve_b: u64,

    /// Total LP tokens in circulation
    pub lp_supply: u64,

    /// Pool bump seed for PDA signing
    pub bump: u8,

    /// LP mint bump seed
    pub lp_mint_bump: u8,

    /// Vault A bump seed
    pub vault_a_bump: u8,

    /// Vault B bump seed
    pub vault_b_bump: u8,

    /// Whether the pool is locked (disables deposit/withdraw)
    pub locked: bool,
}

impl Pool {
    /// Space required: discriminator + all fields
    pub const LEN: usize = 8   // discriminator
        + 32   // authority
        + 32   // mint_a
        + 32   // mint_b
        + 32   // vault_a
        + 32   // vault_b
        + 32   // lp_mint
        + 2    // fee_bps
        + 8    // reserve_a
        + 8    // reserve_b
        + 8    // lp_supply
        + 1    // bump
        + 1    // lp_mint_bump
        + 1    // vault_a_bump
        + 1    // vault_b_bump
        + 1;   // locked
}
