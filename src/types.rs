use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackstopTier {
    FirstLoss,
    SecondLoss,
    ThirdLoss,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub backstop: Address,
    pub blnt: Address,
    pub first_blnt_first: bool,
    pub owner: Address,
    pub first_lp: Address,
    pub manager: Address,
    pub pool: Address,
    pub second_blnt_first: bool,
    pub second_lp: Address,
    pub source_backfill: Address,
    pub total_allocation: i128,
    pub usdc: Address,
    pub vesting_end: u64,
    pub vesting_start: u64,
    pub xlm: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimState {
    pub allocation: i128,
    pub claimed: i128,
    pub payable: i128,
    pub unclaimed: i128,
    pub vested: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Totals {
    pub allocated: i128,
    pub claimed: i128,
    pub custody: i128,
    pub funding_gap: i128,
    pub outstanding: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimAndFundDetails {
    pub backstop_budget: i128,
    pub backstop_unspent: i128,
    pub owner_retained: i128,
    pub first_backstop_shares: i128,
    pub first_blnt: i128,
    pub first_emission_lp: i128,
    pub first_lp: i128,
    pub first_xlm: i128,
    pub grant_claimed: i128,
    pub lockup_funded: i128,
    pub second_backstop_shares: i128,
    pub second_blnt: i128,
    pub second_emission_lp: i128,
    pub second_lp: i128,
    pub second_usdc: i128,
}
