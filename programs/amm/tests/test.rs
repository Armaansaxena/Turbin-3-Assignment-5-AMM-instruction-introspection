//! AMM Program — Integration & Unit Tests
//!
//! Structure:
//!   tests/
//!     test.rs               ← this file (test harness + common helpers)
//!     ix_handler/
//!       mod.rs
//!       init.rs             ← initialize instruction tests
//!       deposit.rs          ← deposit instruction tests
//!       withdraw.rs         ← withdraw instruction tests
//!       swap.rs             ← swap instruction tests

mod ix_handler;

// ── Common test utilities ────────────────────────────────────────────────────

pub mod tests {
    pub mod common {
        use anchor_lang::prelude::*;

        /// A helper struct representing a test pool setup.
        /// In a real BanksClient setup, this would hold the program test context.
        pub struct TestPool {
            pub mint_a: Pubkey,
            pub mint_b: Pubkey,
            pub pool:   Pubkey,
            pub lp_mint: Pubkey,
            pub vault_a: Pubkey,
            pub vault_b: Pubkey,
        }

        /// Compute the expected AMM output for a swap using the constant product formula.
        ///
        /// # Arguments
        /// * `reserve_in`  — current reserve of input token
        /// * `reserve_out` — current reserve of output token
        /// * `amount_in`   — amount of input token to swap
        /// * `fee_bps`     — fee in basis points
        pub fn compute_swap_output(
            reserve_in:  u64,
            reserve_out: u64,
            amount_in:   u64,
            fee_bps:     u64,
        ) -> u64 {
            let amount_in_after_fee = amount_in * (10_000 - fee_bps) / 10_000;
            (reserve_out as u128 * amount_in_after_fee as u128
                / (reserve_in as u128 + amount_in_after_fee as u128)) as u64
        }

        /// Compute initial LP tokens minted from a first deposit.
        /// Returns geometric_mean(amount_a, amount_b) - MINIMUM_LIQUIDITY
        pub fn compute_initial_lp(amount_a: u64, amount_b: u64) -> u64 {
            let product = amount_a as u128 * amount_b as u128;
            let sqrt = integer_sqrt(product);
            sqrt.saturating_sub(1_000) as u64 // MINIMUM_LIQUIDITY = 1_000
        }

        /// Compute proportional LP tokens for a subsequent deposit.
        pub fn compute_lp_for_deposit(
            amount_a:   u64,
            reserve_a:  u64,
            lp_supply:  u64,
        ) -> u64 {
            (amount_a as u128 * lp_supply as u128 / reserve_a as u128) as u64
        }

        // Integer square root
        pub fn integer_sqrt(n: u128) -> u128 {
            if n == 0 { return 0; }
            let mut x = n;
            let mut y = (x + 1) / 2;
            while y < x {
                x = y;
                y = (x + n / x) / 2;
            }
            x
        }
    }
}

// ── Smoke tests for math helpers ─────────────────────────────────────────────

#[cfg(test)]
mod math_tests {
    use super::tests::common::*;

    #[test]
    fn test_compute_swap_output_basic() {
        // reserve_a=1_000_000, reserve_b=1_000_000, amount_in=100_000, fee=30bps
        let out = compute_swap_output(1_000_000, 1_000_000, 100_000, 30);
        // amount_in_after_fee = 99_700
        // out = 1_000_000 * 99_700 / 1_099_700 ≈ 90_698
        assert!(out > 90_000 && out < 91_000, "expected ~90_698, got {}", out);
    }

    #[test]
    fn test_compute_swap_output_zero_fee() {
        let out = compute_swap_output(1_000_000, 1_000_000, 100_000, 0);
        // out = 1_000_000 * 100_000 / 1_100_000 ≈ 90_909
        assert!(out > 90_500 && out < 91_500, "expected ~90_909, got {}", out);
    }

    #[test]
    fn test_compute_initial_lp() {
        // sqrt(1_000_000 * 4_000_000) = sqrt(4e12) = 2_000_000
        let lp = compute_initial_lp(1_000_000, 4_000_000);
        assert_eq!(lp, 1_999_000); // 2_000_000 - MINIMUM_LIQUIDITY(1_000)
    }

    #[test]
    fn test_integer_sqrt_perfect_square() {
        assert_eq!(integer_sqrt(9), 3);
        assert_eq!(integer_sqrt(100), 10);
        assert_eq!(integer_sqrt(1_000_000), 1_000);
    }

    #[test]
    fn test_integer_sqrt_zero() {
        assert_eq!(integer_sqrt(0), 0);
    }

    #[test]
    fn test_constant_product_invariant() {
        // After a swap, k must not decrease (fee keeps extra value in pool)
        let reserve_a: u64 = 1_000_000;
        let reserve_b: u64 = 1_000_000;
        let k_before = reserve_a as u128 * reserve_b as u128;

        let amount_in: u64 = 100_000;
        let amount_out = compute_swap_output(reserve_a, reserve_b, amount_in, 30);

        let new_reserve_a = reserve_a + amount_in;
        let new_reserve_b = reserve_b - amount_out;
        let k_after = new_reserve_a as u128 * new_reserve_b as u128;

        assert!(k_after >= k_before, "constant product invariant violated: k_after={} < k_before={}", k_after, k_before);
    }
}
