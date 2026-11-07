use soroban_sdk::{contracttype, unwrap::UnwrapOptimized, Address, Env, Map, Vec};

use crate::Config;

const ONE_DAY_LEDGERS: u32 = 17_280;
const TTL_THRESHOLD: u32 = 90 * ONE_DAY_LEDGERS;
const TTL_BUMP: u32 = 180 * ONE_DAY_LEDGERS;

#[derive(Clone)]
#[contracttype]
enum Key {
    Allocations,
    Config,
    Lock,
    Progress,
    TotalClaimed,
}

pub fn extend(e: &Env) {
    e.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_BUMP);
}

pub fn get_config(e: &Env) -> Config {
    e.storage().instance().get(&Key::Config).unwrap_optimized()
}

pub fn set_config(e: &Env, config: &Config) {
    e.storage().instance().set(&Key::Config, config);
}

pub fn get_allocations(e: &Env) -> Map<Address, i128> {
    e.storage()
        .instance()
        .get(&Key::Allocations)
        .unwrap_optimized()
}

pub fn set_allocations(e: &Env, allocations: &Map<Address, i128>) {
    e.storage().instance().set(&Key::Allocations, allocations);
}

pub fn allocation_list(e: &Env) -> Vec<(Address, i128)> {
    let allocations = get_allocations(e);
    let mut result = Vec::new(e);
    for entry in allocations.iter() {
        result.push_back(entry);
    }
    result
}

pub fn get_progress(e: &Env) -> Map<Address, i128> {
    e.storage()
        .instance()
        .get(&Key::Progress)
        .unwrap_optimized()
}

pub fn set_progress(e: &Env, progress: &Map<Address, i128>) {
    e.storage().instance().set(&Key::Progress, progress);
}

pub fn get_total_claimed(e: &Env) -> i128 {
    e.storage().instance().get(&Key::TotalClaimed).unwrap_or(0)
}

pub fn set_total_claimed(e: &Env, claimed: i128) {
    e.storage().instance().set(&Key::TotalClaimed, &claimed);
}

pub fn is_locked(e: &Env) -> bool {
    e.storage().instance().get(&Key::Lock).unwrap_or(false)
}

pub fn set_lock(e: &Env, locked: bool) {
    e.storage().instance().set(&Key::Lock, &locked);
}
