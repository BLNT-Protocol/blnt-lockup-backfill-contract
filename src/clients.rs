use soroban_sdk::{contractclient, Address, Env};

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
}
