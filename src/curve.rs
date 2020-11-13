//! Swap calculations and curve implementations

use crate::{bn::U256, error::SwapError};

/// Number of coins
const N_COINS: u64 = 2;

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source_amount: U256,
    /// New amount of destination token
    pub new_destination_amount: U256,
    /// Amount of destination token swapped
    pub amount_swapped: U256,
}

/// The StableSwap invariant calculator.
pub struct StableSwap {
    /// Amplification coefficient (A)
    pub amp_factor: U256,
}

impl StableSwap {
    /// New StableSwap calculator
    pub fn new(amp_factor_u64: u64) -> Result<StableSwap, SwapError> {
        let amp_factor = U256::from(amp_factor_u64);
        Ok(Self { amp_factor })
    }

    fn compute_next_d(&self, initial_d: U256, d_p: U256, sum_x: U256) -> Option<U256> {
        let ann = self.amp_factor.checked_mul(N_COINS.into())?;
        let leverage = ann.checked_mul(sum_x)?;
        let numerator = initial_d.checked_mul(d_p.checked_mul(N_COINS.into())?.checked_add(leverage)?)?;
        let denominator = initial_d.checked_mul(ann.checked_sub(1.into())?)?.checked_add(d_p.checked_mul((N_COINS + 1).into())?)?;
        numerator.checked_div(denominator)
    }

    /// Compute stable swap invariant (D)
    /// Equation:
    /// A * sum(x_i) * n**n + D = A * D * n**n + D**(n+1) / (n**n * prod(x_i))
    pub fn compute_d(&self, amount_a: U256, amount_b: U256) -> Option<U256> {
        let sum_x = amount_a.checked_add(amount_b)?; // sum(x_i), a.k.a S
        if sum_x == 0.into() {
            Some(0.into())
        } else {
            let amount_a_times_coins = amount_a.checked_mul(N_COINS.into())?;
            let amount_b_times_coins = amount_b.checked_mul(N_COINS.into())?;

            // Newton's method to approximate D
            let mut d_prev: U256;
            let mut d = sum_x;
            for _ in 0..256 {
                let mut d_p = d;
                d_p = d_p * d / amount_a_times_coins;
                d_p = d_p * d / amount_b_times_coins;
                d_prev = d;
                d = self.compute_next_d(d, d_p, sum_x)?;
                // Equality with the precision of 1
                if d > d_p {
                    if d.checked_sub(d_prev)? <= 1.into() {
                        break;
                    }
                } else if d_prev.checked_sub(d)? <= 1.into() {
                    break;
                }
            }

            Some(d)
        }
    }

    /// Compute swap amount `y` in proportion to `x`
    /// Solve for y:
    /// y**2 + y * (sum' - (A*n**n - 1) * D / (A * n**n)) = D ** (n + 1) / (n ** (2 * n) * prod' * A)
    /// y**2 + b*y = c
    pub fn compute_y(&self, x: U256, d: U256) -> Option<U256> {
        // TODO: Handle overflows
        let ann: U256 = self.amp_factor.checked_mul(N_COINS.into())?; // A * n

        // sum' = prod' = x
        // c =  D ** (n + 1) / (n ** (2 * n) * prod' * A)
        let c = d.checked_pow(3.into())?.checked_div(x.checked_mul(N_COINS.into())?.checked_mul(N_COINS.into())?.checked_mul(ann)?)?;
        // b = sum' - (A*n**n - 1) * D / (A * n**n)
        let b = d.checked_div(ann)?.checked_add(x)?; // d is subtracted on line 82

        // Solve for y by approximating: y**2 + b*y = c
        let mut y_prev: U256;
        let mut y = d;
        for _ in 0..128 {
            y_prev = y;
            // y = (y * y + c) / (2.into() * y + b - d);
            let y_numerator = y.checked_pow(2.into())?.checked_add(c)?;
            let y_denominator = y.checked_mul(2.into())?.checked_add(b)?.checked_sub(d)?;
            y = y_numerator.checked_div(y_denominator)?;
            if y > y_prev {
                if y.checked_sub(y_prev)? <= 1.into() {
                    break;
                }
            } else if y_prev.checked_sub(y)? <= 1.into() {
                break;
            }
        }

        Some(y)
    }

    /// Compute SwapResult after an exchange
    pub fn swap_to(
        &self,
        source_amount: U256,
        swap_source_amount: U256,
        swap_destination_amount: U256,
        fee_numerator: U256,
        fee_denominator: U256,
    ) -> Option<SwapResult> {
        let y = self.compute_y(
            swap_source_amount.checked_add(source_amount)?,
            self.compute_d(swap_source_amount, swap_destination_amount)?,
        )?;
        let dy = swap_destination_amount.checked_sub(y)?;
        let dy_fee = dy
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;

        let amount_swapped = dy.checked_sub(dy_fee)?;
        let new_destination_amount = swap_destination_amount.checked_sub(amount_swapped)?;
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;

        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
        })
    }
}

/// Conversions for pool tokens, how much to deposit / withdraw, along with
/// proper initialization
pub struct PoolTokenConverter {
    /// Total supply
    pub supply: U256,
    /// Token A amount
    pub token_a: U256,
    /// Token B amount
    pub token_b: U256,
}

impl PoolTokenConverter {
    /// Create a converter based on existing market information
    pub fn new(supply: U256, token_a: U256, token_b: U256) -> Self {
        Self {
            supply,
            token_a,
            token_b,
        }
    }

    /// A tokens for pool tokens
    pub fn token_a_rate(&self, pool_tokens: U256) -> Option<U256> {
        pool_tokens
            .checked_mul(self.token_a)?
            .checked_div(self.supply)
    }

    /// B tokens for pool tokens
    pub fn token_b_rate(&self, pool_tokens: U256) -> Option<U256> {
        pool_tokens
            .checked_mul(self.token_b)?
            .checked_div(self.supply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use sim::{Model, MODEL_FEE_DENOMINATOR, MODEL_FEE_NUMERATOR};

    fn check_pool_token_a_rate(
        token_a: U256,
        token_b: U256,
        deposit: U256,
        supply: U256,
        expected: Option<U256>,
    ) {
        let calculator = PoolTokenConverter::new(supply, token_a, token_b);
        assert_eq!(calculator.token_a_rate(deposit), expected);
        assert_eq!(calculator.supply, supply);
    }

    #[test]
    fn issued_tokens() {
        check_pool_token_a_rate(2.into(), 50.into(), 5.into(), 10.into(), Some(1.into()));
        check_pool_token_a_rate(10.into(), 10.into(), 5.into(), 10.into(), Some(5.into()));
        check_pool_token_a_rate(5.into(), 100.into(), 5.into(), 10.into(), Some(2.into()));
        check_pool_token_a_rate(5.into(), U256::MAX, 5.into(), 10.into(), Some(2.into()));
        check_pool_token_a_rate(U256::MAX, U256::MAX, 5.into(), 10.into(), None);
    }

    fn check_d(model: &Model, amount_a: u64, amount_b: u64) -> U256 {
        let swap = StableSwap {
            amp_factor: U256::from(model.amp_factor),
        };
        let d = swap.compute_d(U256::from(amount_a), U256::from(amount_b)).unwrap();
        assert_eq!(d, model.sim_d().into());
        d
    }

    fn check_y(model: &Model, x: u64, d: U256) {
        let swap = StableSwap {
            amp_factor: U256::from(model.amp_factor),
        };
        assert_eq!(swap.compute_y(x.into(), d).unwrap(), model.sim_y(0, 1, x).into())
    }

    #[test]
    fn test_curve_math() {
        let n_coin = 2;

        let model_no_balance = Model::new(1, vec![0, 0], n_coin);
        check_d(&model_no_balance, 0, 0);

        let amount_a = 1_000_000_000;
        let amount_b = 1_000_000_000;
        let model_a1 = Model::new(1, vec![amount_a, amount_b], n_coin);
        let d = check_d(&model_a1, amount_a.into(), amount_b.into());
        check_y(&model_a1, 1, d);
        check_y(&model_a1, 1000, d);
        check_y(&model_a1, amount_a.into(), d);

        let model_a100 = Model::new(100, vec![amount_a, amount_b], n_coin);
        let d = check_d(&model_a100, amount_a.into(), amount_b.into());
        check_y(&model_a100, 1, d);
        check_y(&model_a100, 1000, d);
        check_y(&model_a100, amount_a.into(), d);

        let model_a1000 = Model::new(1000, vec![amount_a, amount_b], n_coin);
        let d = check_d(&model_a1000, amount_a.into(), amount_b.into());
        check_y(&model_a1000, 1, d);
        check_y(&model_a1000, 1000, d);
        check_y(&model_a1000, amount_a.into(), d);
    }

    #[test]
    fn test_curve_math_with_random_inputs() {
        let mut rng = rand::thread_rng();

        let n_coin = 2;
        let amp_factor: u64 = rng.gen_range(1, 10_000);
        let amount_a: u64 = rng.gen_range(1, 10_000_000_000);
        let amount_b: u64 = rng.gen_range(1, 10_000_000_000);
        println!("testing curve_math_with_random_inputs:");
        println!(
            "amount_a: {}, amount_b: {}, amp_factor: {}",
            amount_a, amount_b, amp_factor
        );

        let model = Model::new(amp_factor, vec![amount_a, amount_b], n_coin);
        let d = check_d(&model, amount_a.into(), amount_b.into());
        check_y(&model, rng.gen_range(0, amount_a), d);
    }

    fn check_swap(
        amp_factor: u64,
        source_amount: u64,
        swap_source_amount: u64,
        swap_destination_amount: u64,
    ) {
        let n_coin = 2;
        let swap = StableSwap { amp_factor: amp_factor.into() };
        let result = swap
            .swap_to(
                source_amount.into(),
                swap_source_amount.into(),
                swap_destination_amount.into(),
                MODEL_FEE_NUMERATOR.into(),
                MODEL_FEE_DENOMINATOR.into(),
            )
            .unwrap();
        let model = Model::new(
            amp_factor,
            vec![swap_source_amount, swap_destination_amount],
            n_coin,
        );

        assert_eq!(
            result.amount_swapped,
            model.sim_exchange(0, 1, source_amount).into()
        );
        assert_eq!(result.new_source_amount, (swap_source_amount + source_amount).into());
        assert_eq!(
            result.new_destination_amount,
            U256::from(swap_destination_amount) - result.amount_swapped
        );
    }

    #[test]
    fn test_swap_calculation() {
        let source_amount: u64 = 10_000_000_000;
        let swap_source_amount: u64 = 50_000_000_000;
        let swap_destination_amount: u64 = 50_000_000_000;

        check_swap(
            1,
            source_amount,
            swap_source_amount,
            swap_destination_amount,
        );
        check_swap(
            10,
            source_amount,
            swap_source_amount,
            swap_destination_amount,
        );
        check_swap(
            100,
            source_amount,
            swap_source_amount,
            swap_destination_amount,
        );
        check_swap(
            1000,
            source_amount,
            swap_source_amount,
            swap_destination_amount,
        );
        check_swap(
            10000,
            source_amount,
            swap_source_amount,
            swap_destination_amount,
        );
    }

    #[test]
    fn test_swap_calculation_with_random_inputs() {
        let mut rng = rand::thread_rng();

        let amp_factor: u64 = rng.gen_range(1, 10_000);
        let source_amount: u64 = rng.gen_range(1, 10_000_000_000);
        let swap_source_amount: u64 = rng.gen_range(1, 10_000_000_000);
        let swap_destination_amount: u64 = rng.gen_range(1, 10_000_000_000);
        println!("testing swap_calculation_with_random_inputs:");
        println!(
            "amp_factor: {}, source_amount: {}, swap_source_amount: {}, swap_destination_amount: {}",
            amp_factor, source_amount, swap_source_amount, swap_destination_amount
        );

        check_swap(
            amp_factor,
            source_amount,
            swap_source_amount,
            swap_destination_amount,
        );
    }
}
