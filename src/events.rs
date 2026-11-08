use soroban_sdk::{contractevent, Address, Env};

use crate::types::ClaimAndFundDetails;

#[contractevent(topics = ["fund"], data_format = "vec")]
struct FundEvent {
    #[topic]
    from: Address,
    amount: i128,
    balance: i128,
}

#[contractevent(topics = ["claim"], data_format = "vec")]
struct ClaimEvent {
    #[topic]
    beneficiary: Address,
    amount: i128,
    total_claimed: i128,
}

#[contractevent(topics = ["claim_fund"], data_format = "single-value")]
struct ClaimAndFundEvent {
    details: ClaimAndFundDetails,
}

#[contractevent(topics = ["expire"], data_format = "vec")]
struct ExpireClaimsEvent {
    #[topic]
    owner: Address,
    amount: i128,
    total_claimed: i128,
}

pub fn fund(e: &Env, from: Address, amount: i128, balance: i128) {
    FundEvent {
        from,
        amount,
        balance,
    }
    .publish(e);
}

pub fn claim(e: &Env, beneficiary: Address, amount: i128, total_claimed: i128) {
    ClaimEvent {
        beneficiary,
        amount,
        total_claimed,
    }
    .publish(e);
}

pub fn claim_and_fund(e: &Env, details: ClaimAndFundDetails) {
    ClaimAndFundEvent { details }.publish(e);
}

pub fn expire_claims(e: &Env, owner: Address, amount: i128, total_claimed: i128) {
    ExpireClaimsEvent {
        owner,
        amount,
        total_claimed,
    }
    .publish(e);
}
