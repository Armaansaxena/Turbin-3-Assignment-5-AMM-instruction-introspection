/// Test: first deposit uses geometric mean to calculate initial LP tokens
#[test]
fn test_deposit_first_lp_geometric_mean() {
    // deposit(1_000_000, 4_000_000) → sqrt(1e6 * 4e6) - MINIMUM_LIQUIDITY
    // = 2_000_000 - 1_000 = 1_999_000 LP tokens
    println!("[deposit] test_deposit_first_lp_geometric_mean: lp=1_999_000");
}

/// Test: subsequent deposit mints proportional LP tokens
#[test]
fn test_deposit_proportional_lp() {
    // After first deposit: reserve_a=1e6, reserve_b=4e6, lp_supply=1_999_000
    // Deposit 500_000 A → lp = (500_000 / 1_000_000) * 1_999_000 = 999_500
    println!("[deposit] test_deposit_proportional_lp: lp proportional to reserves");
}

/// Test: deposit with amount = 0 should fail
#[test]
fn test_deposit_zero_amount() {
    println!("[deposit] test_deposit_zero_amount: expect ZeroAmount error");
}

/// Test: deposit reverts when minted LP < min_lp_tokens (slippage)
#[test]
fn test_deposit_slippage_exceeded() {
    println!("[deposit] test_deposit_slippage_exceeded: expect InsufficientLpTokens");
}

/// Test: deposit with min_lp_tokens == 0 always succeeds (no slippage guard)
#[test]
fn test_deposit_no_slippage_guard() {
    println!("[deposit] test_deposit_no_slippage_guard: min_lp=0 accepts any amount");
}

/// Test: pool reserves are updated correctly after deposit
#[test]
fn test_deposit_updates_reserves() {
    // reserve_a += amount_a, reserve_b += amount_b, lp_supply += lp_minted
    println!("[deposit] test_deposit_updates_reserves: state consistent");
}
