use soroban_sdk::{contractevent, Address, Env};

use crate::RouteResult;

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

#[contractevent(topics = ["route"], data_format = "single-value")]
struct RouteEvent {
    result: RouteResult,
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

pub fn route(e: &Env, result: RouteResult) {
    RouteEvent { result }.publish(e);
}
