use crate::{
    clients::{BackstopManagerClient, SourceBackfillClient},
    errors::LockupBackfillError,
    events, storage,
    types::{ClaimAndFundDetails, ClaimState, Config, Totals},
    VESTING_DURATION_SECONDS,
};
use soroban_sdk::{
    contract, contractimpl, panic_with_error, token::TokenClient, Address, Env, Map, Vec,
};

#[contract]
pub struct BlntLockupBackfillContract;

fn require_unlocked(e: &Env) {
    if storage::is_locked(e) {
        panic_with_error!(e, LockupBackfillError::ReentrantCall);
    }
}

fn checked_add(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_add(right)
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::Overflow))
}

fn checked_sub(e: &Env, left: i128, right: i128) -> i128 {
    left.checked_sub(right)
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::Overflow))
}

fn require_claims_open(e: &Env) {
    if storage::claims_closed(e) {
        panic_with_error!(e, LockupBackfillError::ClaimsClosed);
    }
}

fn transfer(e: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
    let client = TokenClient::new(e, token);
    let from_before = client.balance(from);
    let to_before = client.balance(to);
    client.transfer(from, to, &amount);
    if from_before.checked_sub(client.balance(from)) != Some(amount)
        || client.balance(to).checked_sub(to_before) != Some(amount)
    {
        panic_with_error!(e, LockupBackfillError::BalanceMismatch);
    }
}

fn vested_amount(e: &Env, allocation: i128, now: u64, start: u64, end: u64) -> i128 {
    if now <= start {
        return 0;
    }
    if now >= end {
        return allocation;
    }
    let elapsed = now
        .checked_sub(start)
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::Overflow));
    let duration = end
        .checked_sub(start)
        .filter(|duration| *duration > 0)
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::InvalidVesting));
    allocation
        .checked_mul(i128::from(elapsed))
        .and_then(|value| value.checked_div(i128::from(duration)))
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::Overflow))
}

fn claim_state(e: &Env, beneficiary: &Address) -> ClaimState {
    let config = storage::get_config(e);
    let allocation = storage::get_allocations(e)
        .get(beneficiary.clone())
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::NoClaim));
    let claimed = storage::get_progress(e)
        .get(beneficiary.clone())
        .unwrap_or(0);
    let vested = vested_amount(
        e,
        allocation,
        e.ledger().timestamp(),
        config.vesting_start,
        config.vesting_end,
    );
    let unclaimed = checked_sub(e, allocation, claimed);
    let vested_unclaimed = checked_sub(e, vested, claimed);
    let custody = TokenClient::new(e, &config.blnt).balance(&e.current_contract_address());
    ClaimState {
        allocation,
        claimed,
        payable: if !storage::claims_closed(e) {
            vested_unclaimed.min(custody)
        } else {
            0
        },
        unclaimed,
        vested,
    }
}

fn validate_manager(e: &Env, config: &Config) {
    let client = BackstopManagerClient::new(e, &config.manager);
    if client.owner() != config.owner {
        panic_with_error!(e, LockupBackfillError::InvalidManager);
    }
}

#[contractimpl]
impl BlntLockupBackfillContract {
    pub fn __constructor(
        e: Env,
        blnt: Address,
        source_backfill: Address,
        owner: Address,
        manager: Address,
        allocation_list: Vec<(Address, i128)>,
    ) {
        let contract = e.current_contract_address();
        if owner == contract
            || manager == contract
            || source_backfill == contract
            || TokenClient::new(&e, &blnt).decimals() != 7
        {
            panic_with_error!(&e, LockupBackfillError::InvalidConfig);
        }

        storage::set_lock(&e, true);
        let source = SourceBackfillClient::new(&e, &source_backfill);
        if source.get_blnt_token() != blnt {
            panic_with_error!(&e, LockupBackfillError::InvalidToken);
        }
        let vesting_start = source.get_grant_vesting_start();
        let vesting_end = source.get_grant_vesting_end();
        if vesting_start > e.ledger().timestamp()
            || vesting_end.checked_sub(vesting_start) != Some(VESTING_DURATION_SECONDS)
        {
            panic_with_error!(&e, LockupBackfillError::InvalidVesting);
        }
        let mut allocations = Map::new(&e);
        let mut total = 0_i128;
        for (beneficiary, amount) in allocation_list.iter() {
            if beneficiary == contract || amount <= 0 {
                panic_with_error!(&e, LockupBackfillError::InvalidAllocation);
            }
            if allocations.contains_key(beneficiary.clone()) {
                panic_with_error!(&e, LockupBackfillError::DuplicateBeneficiary);
            }
            total = checked_add(&e, total, amount);
            allocations.set(beneficiary, amount);
        }
        let config = Config {
            blnt,
            owner,
            manager,
            source_backfill,
            total_allocation: total,
            vesting_end,
            vesting_start,
        };
        validate_manager(&e, &config);
        storage::set_config(&e, &config);
        storage::set_allocations(&e, &allocations);
        storage::set_claims_closed(&e, false);
        storage::set_progress(&e, &Map::new(&e));
        storage::set_total_claimed(&e, 0);
        storage::set_lock(&e, false);
        storage::extend(&e);
    }

    pub fn fund(e: Env, from: Address, amount: i128) -> i128 {
        storage::extend(&e);
        require_unlocked(&e);
        if amount <= 0 || from == e.current_contract_address() {
            panic_with_error!(&e, LockupBackfillError::InvalidAmount);
        }
        from.require_auth();
        let config = storage::get_config(&e);
        require_claims_open(&e);
        storage::set_lock(&e, true);
        let contract = e.current_contract_address();
        let balance = TokenClient::new(&e, &config.blnt).balance(&contract);
        let outstanding = checked_sub(&e, config.total_allocation, storage::get_total_claimed(&e));
        if checked_add(&e, balance, amount) > outstanding {
            panic_with_error!(&e, LockupBackfillError::Overfunded);
        }

        transfer(&e, &config.blnt, &from, &contract, amount);
        storage::set_lock(&e, false);
        let new_balance = checked_add(&e, balance, amount);
        events::fund(&e, from, amount, new_balance);
        amount
    }

    pub fn claim(e: Env, beneficiary: Address) -> i128 {
        storage::extend(&e);
        require_unlocked(&e);
        beneficiary.require_auth();
        let config = storage::get_config(&e);
        require_claims_open(&e);
        storage::set_lock(&e, true);
        let state = claim_state(&e, &beneficiary);
        if state.payable <= 0 {
            panic_with_error!(&e, LockupBackfillError::NothingClaimable);
        }

        let mut progress = storage::get_progress(&e);
        let beneficiary_claimed = checked_add(&e, state.claimed, state.payable);
        let total_claimed = checked_add(&e, storage::get_total_claimed(&e), state.payable);
        progress.set(beneficiary.clone(), beneficiary_claimed);

        storage::set_progress(&e, &progress);
        storage::set_total_claimed(&e, total_claimed);
        transfer(
            &e,
            &config.blnt,
            &e.current_contract_address(),
            &beneficiary,
            state.payable,
        );
        storage::set_lock(&e, false);
        events::claim(&e, beneficiary, state.payable, total_claimed);
        state.payable
    }

    pub fn claim_and_fund(e: Env) -> i128 {
        storage::extend(&e);
        require_unlocked(&e);
        let config = storage::get_config(&e);
        config.owner.require_auth();

        storage::set_lock(&e, true);
        validate_manager(&e, &config);
        let blnt = TokenClient::new(&e, &config.blnt);
        let contract = e.current_contract_address();
        let owner_before = blnt.balance(&config.owner);
        let contract_before = blnt.balance(&contract);
        let manager_before = blnt.balance(&config.manager);

        let source_claimed =
            SourceBackfillClient::new(&e, &config.source_backfill).claim_grant(&config.owner);

        let manager_amount = source_claimed / 3;
        let post_manager = checked_sub(&e, source_claimed, manager_amount);
        let outstanding = checked_sub(&e, config.total_allocation, storage::get_total_claimed(&e));
        let funding_gap = outstanding.saturating_sub(contract_before);
        let lockup_amount = if storage::claims_closed(&e) {
            0
        } else {
            funding_gap.min(post_manager)
        };

        if lockup_amount > 0 {
            blnt.transfer(&config.owner, &contract, &lockup_amount);
        }
        if manager_amount > 0 {
            blnt.transfer(&config.owner, &config.manager, &manager_amount);
        }
        let owner_retained = checked_sub(
            &e,
            checked_sub(&e, source_claimed, lockup_amount),
            manager_amount,
        );
        if blnt.balance(&contract).checked_sub(contract_before) != Some(lockup_amount)
            || blnt.balance(&config.manager).checked_sub(manager_before) != Some(manager_amount)
            || blnt.balance(&config.owner).checked_sub(owner_before) != Some(owner_retained)
        {
            panic_with_error!(&e, LockupBackfillError::BalanceMismatch);
        }
        storage::set_lock(&e, false);

        events::claim_and_fund(
            &e,
            ClaimAndFundDetails {
                grant_claimed: source_claimed,
                lockup_funded: lockup_amount,
                manager_funded: manager_amount,
                owner_retained,
            },
        );
        source_claimed
    }

    pub fn expire_claims(e: Env) -> i128 {
        storage::extend(&e);
        require_unlocked(&e);
        let config = storage::get_config(&e);
        config.owner.require_auth();
        if storage::claims_closed(&e) {
            panic_with_error!(&e, LockupBackfillError::ClaimsClosed);
        }

        storage::set_lock(&e, true);
        storage::set_claims_closed(&e, true);
        let contract = e.current_contract_address();
        let amount = TokenClient::new(&e, &config.blnt).balance(&contract);
        if amount > 0 {
            transfer(&e, &config.blnt, &contract, &config.owner, amount);
        }
        storage::set_lock(&e, false);
        events::expire_claims(&e, config.owner, amount, storage::get_total_claimed(&e));
        amount
    }

    pub fn claims_closed(e: Env) -> bool {
        storage::extend(&e);
        storage::claims_closed(&e)
    }

    pub fn get_config(e: Env) -> Config {
        storage::extend(&e);
        storage::get_config(&e)
    }

    pub fn get_allocations(e: Env) -> Vec<(Address, i128)> {
        storage::extend(&e);
        storage::allocation_list(&e)
    }

    pub fn get_claim(e: Env, beneficiary: Address) -> ClaimState {
        storage::extend(&e);
        require_unlocked(&e);
        storage::set_lock(&e, true);
        let state = claim_state(&e, &beneficiary);
        storage::set_lock(&e, false);
        state
    }

    pub fn get_totals(e: Env) -> Totals {
        storage::extend(&e);
        require_unlocked(&e);
        storage::set_lock(&e, true);
        let config = storage::get_config(&e);
        let claimed = storage::get_total_claimed(&e);
        let outstanding = checked_sub(&e, config.total_allocation, claimed);
        let custody = TokenClient::new(&e, &config.blnt).balance(&e.current_contract_address());
        let totals = Totals {
            allocated: config.total_allocation,
            claimed,
            custody,
            funding_gap: outstanding.saturating_sub(custody),
            outstanding,
        };
        storage::set_lock(&e, false);
        totals
    }
}
