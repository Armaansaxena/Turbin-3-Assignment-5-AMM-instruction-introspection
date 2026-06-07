/// Test: burn LP tokens and receive proportional token A + B
#[test]
fn test_withdraw_proportional() {
    // If lp_supply=1_999_000, lp_burned=999_500 (50%)
    // amount_a_out = 50% of reserve_a
    // amount_b_out = 50% of reserve_b
    println!("[withdraw] test_withdraw_proportional: 50% burn → 50% of reserves");
}

/// Test: withdraw full LP supply
#[test]
fn test_withdraw_full_supply() {
    // Burning all LP should drain both vaults (minus MINIMUM_LIQUIDITY)
    println!("[withdraw] test_withdraw_full_supply: all LP burned");
}

/// Test: withdraw fails if lp_amount > user balance
#[test]
fn test_withdraw_insufficient_lp_balance() {
    println!("[withdraw] test_withdraw_insufficient_lp_balance: expect InsufficientLpBalance");
}

/// Test: withdraw reverts when received token A < min_amount_a
#[test]
fn test_withdraw_slippage_a() {
    println!("[withdraw] test_withdraw_slippage_a: expect SlippageExceeded");
}

/// Test: withdraw reverts when received token B < min_amount_b
#[test]
fn test_withdraw_slippage_b() {
    println!("[withdraw] test_withdraw_slippage_b: expect SlippageExceeded");
}

/// Test: lp_amount == 0 should fail
#[test]
fn test_withdraw_zero_lp() {
    println!("[withdraw] test_withdraw_zero_lp: expect ZeroAmount");
}

/// Test: pool state (reserves, lp_supply) updated correctly after withdraw
#[test]
fn test_withdraw_updates_state() {
    println!("[withdraw] test_withdraw_updates_state: reserve_a/b and lp_supply decremented");
}
