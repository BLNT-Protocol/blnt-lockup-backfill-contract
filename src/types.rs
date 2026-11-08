use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub blnt: Address,
    pub owner: Address,
    pub manager: Address,
    pub source_backfill: Address,
    pub total_allocation: i128,
    pub vesting_end: u64,
    pub vesting_start: u64,
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
    pub grant_claimed: i128,
    pub lockup_funded: i128,
    pub manager_funded: i128,
    pub owner_retained: i128,
}
