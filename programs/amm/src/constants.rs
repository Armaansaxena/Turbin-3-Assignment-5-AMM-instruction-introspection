/// Seed for the pool state PDA
pub const POOL_SEED: &[u8] = b"pool";

/// Seed for the LP mint PDA
pub const LP_MINT_SEED: &[u8] = b"lp_mint";

/// Seed for vault A (token A reserve)
pub const VAULT_A_SEED: &[u8] = b"vault_a";

/// Seed for vault B (token B reserve)
pub const VAULT_B_SEED: &[u8] = b"vault_b";

/// Maximum fee in basis points (10%)
pub const MAX_FEE_BPS: u16 = 1_000;

/// Basis points denominator
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Minimum liquidity to permanently lock on first deposit (prevents division by zero)
pub const MINIMUM_LIQUIDITY: u64 = 1_000;

/// LP token decimals
pub const LP_TOKEN_DECIMALS: u8 = 6;
