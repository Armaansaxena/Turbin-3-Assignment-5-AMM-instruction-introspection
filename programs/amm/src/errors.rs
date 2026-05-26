use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Fee basis points exceeds maximum allowed (1000 bps = 10%)")]
    FeeTooHigh,

    #[msg("Slippage tolerance exceeded: output amount is less than minimum")]
    SlippageExceeded,

    #[msg("Zero amount provided — must be greater than 0")]
    ZeroAmount,

    #[msg("Insufficient liquidity in the pool for this operation")]
    InsufficientLiquidity,

    #[msg("LP token amount exceeds user balance")]
    InsufficientLpBalance,

    #[msg("Arithmetic overflow during calculation")]
    MathOverflow,

    #[msg("Pool is already initialized")]
    AlreadyInitialized,

    #[msg("Token mint mismatch: provided mint does not match pool")]
    InvalidMint,

    #[msg("Invalid vault account provided")]
    InvalidVault,

    #[msg("Deposit amounts violate pool ratio (use proportional amounts)")]
    InvalidDepositRatio,

    #[msg("Received LP tokens below minimum requested")]
    InsufficientLpTokens,
}
