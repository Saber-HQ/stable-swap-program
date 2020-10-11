//! Swap calculations and curve implementations

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u64.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u64 = 1_000_000_000; // TODO: Remove

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source_amount: u64,
    /// New amount of destination token
    pub new_destination_amount: u64,
    /// Amount of destination token swapped
    pub amount_swapped: u64,
}

impl SwapResult {
    /// SwapResult for swap from one currency into another, given pool information
    /// and fee
    pub fn swap_to(
        source_amount: u64,
        swap_source_amount: u64,
        swap_destination_amount: u64,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Option<SwapResult> {
        let invariant = swap_source_amount.checked_mul(swap_destination_amount)?;

        // debit the fee to calculate the amount swapped
        let fee = source_amount
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        let new_source_amount_less_fee = swap_source_amount
            .checked_add(source_amount)?
            .checked_sub(fee)?;
        let new_destination_amount = invariant.checked_div(new_source_amount_less_fee)?;
        let amount_swapped = swap_destination_amount.checked_sub(new_destination_amount)?;

        // actually add the whole amount coming in
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
        })
    }
}

/// The StableSwap invariant calculator.
pub struct StableSwap {
    /// Token A
    pub token_a: u64,
    /// Token B
    pub token_b: u64,
    /// Fee numerator
    pub fee_numerator: u64,
    /// Fee denominator
    pub fee_denominator: u64,
}

impl StableSwap {
    /// Swap token a to b
    pub fn swap_a_to_b(&mut self, token_a: u64) -> Option<u64> {
        let result = SwapResult::swap_to(
            token_a,
            self.token_a,
            self.token_b,
            self.fee_numerator,
            self.fee_denominator,
        )?;
        self.token_a = result.new_source_amount;
        self.token_b = result.new_destination_amount;
        Some(result.amount_swapped)
    }

    /// Swap token b to a
    pub fn swap_b_to_a(&mut self, token_b: u64) -> Option<u64> {
        let result = SwapResult::swap_to(
            token_b,
            self.token_b,
            self.token_a,
            self.fee_numerator,
            self.fee_denominator,
        )?;
        self.token_b = result.new_source_amount;
        self.token_a = result.new_destination_amount;
        Some(result.amount_swapped)
    }
}

/// Conversions for pool tokens, how much to deposit / withdraw, along with
/// proper initialization
pub struct PoolTokenConverter {
    /// Amplification coefficient (A)
    pub amp_factor: u64,
    /// Total supply
    pub supply: u64, // TODO: Remove
    /// Token A amount
    pub token_a: u64,
    /// Token B amount
    pub token_b: u64,
}

impl PoolTokenConverter {
    /// Create a converter based on existing market information
    pub fn new_existing(amp_factor: u64, supply: u64, token_a: u64, token_b: u64) -> Self {
        Self {
            amp_factor,
            supply,
            token_a,
            token_b,
        }
    }

    /// Create a converter for a new pool token, no supply present yet.
    /// According to Uniswap, the geometric mean protects the pool creator
    /// in case the initial ratio is off the market.
    pub fn new_pool(amp_factor: u64, token_a: u64, token_b: u64) -> Self {
        let supply = INITIAL_SWAP_POOL_AMOUNT;
        Self {
            amp_factor,
            supply,
            token_a,
            token_b,
        }
    }

    /// A tokens for pool tokens
    pub fn token_a_rate(&self, pool_tokens: u64) -> Option<u64> {
        pool_tokens
            .checked_mul(self.token_a)?
            .checked_div(self.supply)
    }

    /// B tokens for pool tokens
    pub fn token_b_rate(&self, pool_tokens: u64) -> Option<u64> {
        pool_tokens
            .checked_mul(self.token_b)?
            .checked_div(self.supply)
    }

    /// Compute stable swap invariant
    pub fn compute_d(&self, amount_a: u64, amount_b: u64) -> u64 {
        // XXX: Curve uses u256
        let n_coins: u64 = 2; // n
        let sum_x = amount_a + amount_b; // sum(x_i), a.k.a S
        if sum_x == 0 {
            0
        } else {
            let mut d_prev: u64;
            let mut d = sum_x;
            let leverage = self.amp_factor * n_coins; // A * n

            // Newton's method to approximate D
            for _ in 0..63 {
                let mut d_p = d;
                d_p = d_p * d / (amount_a * n_coins);
                d_p = d_p * d / (amount_b * n_coins);
                d_prev = d;
                d = (leverage * sum_x + d_p * n_coins) * d
                    / ((leverage - 1) * d + (n_coins + 1) * d_p);
                // Equality with the precision of 1
                if d > d_p {
                    if d - d_prev <= 1 {
                        break;
                    }
                } else if d_prev - d <= 1 {
                    break;
                }
            }

            d
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_pool_amount() {
        let token_converter = PoolTokenConverter::new_pool(0, 1, 5);
        assert_eq!(token_converter.supply, INITIAL_SWAP_POOL_AMOUNT);
    }

    fn check_pool_token_a_rate(
        amp_factor: u64,
        token_a: u64,
        token_b: u64,
        deposit: u64,
        supply: u64,
        expected: Option<u64>,
    ) {
        let calculator = PoolTokenConverter::new_existing(amp_factor, supply, token_a, token_b);
        assert_eq!(calculator.token_a_rate(deposit), expected);
    }

    #[test]
    fn issued_tokens() {
        check_pool_token_a_rate(1, 2, 50, 5, 10, Some(1));
        check_pool_token_a_rate(1, 10, 10, 5, 10, Some(5));
        check_pool_token_a_rate(1, 5, 100, 5, 10, Some(2));
        check_pool_token_a_rate(1, 5, u64::MAX, 5, 10, Some(2));
        check_pool_token_a_rate(1, u64::MAX, u64::MAX, 5, 10, None);
    }

    #[test]
    fn swap_calculation() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount: u64 = 1000;
        let swap_destination_amount: u64 = 50000;
        let fee_numerator: u64 = 1;
        let fee_denominator: u64 = 100;
        let source_amount: u64 = 100;
        let result = SwapResult::swap_to(
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            fee_numerator,
            fee_denominator,
        )
        .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4505);
        assert_eq!(result.new_destination_amount, 45495);
    }
}
