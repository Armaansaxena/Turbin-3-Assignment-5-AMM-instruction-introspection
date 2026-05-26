# Solana Constant Product AMM

A production-ready Automated Market Maker (AMM) built using the Anchor framework on Solana. This program implements a classic Uniswap-style `x * y = k` constant product formula with liquidity provider (LP) tokens, swap fees, and advanced safety features.

## 🚀 Features

- **Constant Product Logic:** Core `x * y = k` formula for automated pricing.
- **Deterministic PDAs:** Pool addresses are derived from sorted mint addresses to prevent duplicate pools for the same pair.
- **Slippage Protection:** Built-in guards for swaps and liquidity operations.
- **Ratio Enforcement:** Prevents "donations" by ensuring deposits match current pool ratios.
- **LP Token System:** Geometric mean calculation for initial liquidity to prevent price manipulation.
- **Boxed Accounts:** Optimized stack usage to support complex instructions within SBF limits.

## 🛠 Technical Details

- **Fee Structure:** Configurable fee in basis points (max 10%).
- **Precision:** Uses `u128` for internal math to prevent overflow and maintain precision.
- **Account Model:** 
  - `Pool`: Main state account.
  - `Vaults`: PDA-owned token accounts for reserves.
  - `LP Mint`: PDA-controlled mint for liquidity tokens.

## 📦 Installation

1. **Clone the repository:**
   ```bash
   git clone <your-repo-url>
   cd amm
   ```

2. **Install dependencies:**
   ```bash
   npm install
   ```

3. **Build the program:**
   ```bash
   anchor build
   ```

## 🧪 Testing

The project includes a comprehensive TypeScript test suite that verifies the full lifecycle of a pool.

```bash
anchor test
```

*Note: If you encounter port conflicts with the local validator (e.g., Port 8000), ensure no other validator instances are running.*

## 📜 Program Instructions

- `initialize`: Creates a new pool. Mints must be provided in sorted order for deterministic PDA derivation.
- `deposit`: Add liquidity. Mints LP tokens based on the current reserve ratio.
- `swap`: Trade token A for B (or vice-versa) with automated fee deduction.
- `withdraw`: Burn LP tokens to receive a proportional share of pool reserves.

## 🔒 Security

- **Deterministic Seeds:** Prevents pool mirroring.
- **Arithmetic Safety:** All calculations use checked math or `u128` widening.
- **Authority Guards:** PDA-owned vaults ensure tokens can only be moved via program logic.

## 📄 License
MIT
