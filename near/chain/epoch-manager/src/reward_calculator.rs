use num_rational::Rational32;
use near_primitives::types::{AccountId};

#[derive(Clone, Debug)]
pub struct RewardCalculator {
    pub max_inflation_rate: Rational32,
    pub num_blocks_per_year: u64,
    pub epoch_length: u64,
    pub protocol_reward_rate: Rational32,
    pub protocol_treasury_account: AccountId,
    pub num_seconds_per_year: u64,
}
