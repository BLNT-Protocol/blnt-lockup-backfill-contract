use soroban_sdk::{contractevent, Address, Env};

use crate::types::GrantFundingDetails;

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

#[contractevent(topics = ["grant_fund"], data_format = "single-value")]
struct GrantFundingEvent {
    details: GrantFundingDetails,
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

pub fn grant_funding(e: &Env, details: GrantFundingDetails) {
    GrantFundingEvent { details }.publish(e);
}

pub fn expire_claims(e: &Env, owner: Address, amount: i128, total_claimed: i128) {
    ExpireClaimsEvent {
        owner,
        amount,
        total_claimed,
    }
    .publish(e);
}
