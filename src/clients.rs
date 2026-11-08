use soroban_sdk::{contractclient, Address, Env, Vec};

use crate::BackstopTier;

#[allow(dead_code)]
#[contractclient(name = "SourceBackfillClient")]
pub trait SourceBackfill {
    fn claim_grant(e: Env, user: Address) -> i128;
    fn get_blnt_token(e: Env) -> Address;
    fn get_grant_vesting_end(e: Env) -> u64;
    fn get_grant_vesting_start(e: Env) -> u64;
}

#[allow(dead_code)]
#[contractclient(name = "BackstopManagerClient")]
pub trait BackstopManager {
    fn owner(e: Env) -> Address;
    fn backstops(e: Env) -> Vec<Address>;
    fn pools(e: Env) -> Vec<Address>;
    fn c_join_pool(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        pool_amount_out: i128,
        max_amounts_in: Vec<i128>,
    );
    fn b_deposit(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        amount: i128,
    ) -> i128;
}

#[allow(dead_code)]
#[contractclient(name = "BackstopClient")]
pub trait Backstop {
    fn backstop_token(e: Env, tier: BackstopTier, pool: Address) -> Address;
}

#[allow(dead_code)]
#[contractclient(name = "CometClient")]
pub trait Comet {
    fn get_balance(e: Env, token: Address) -> i128;
    fn get_total_supply(e: Env) -> i128;
    fn get_tokens(e: Env) -> Vec<Address>;
}
