use crate::{
    allocations,
    clients::{BackstopClient, BackstopManagerClient, CometClient, SourceBackfillClient},
    errors::LockupBackfillError,
    events, storage,
    types::{BackstopTier, ClaimState, Config, RouteResult, Totals},
    BENEFICIARY_COUNT, TOTAL_ALLOCATION, VESTING_DURATION_SECONDS,
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

fn contains(values: &Vec<Address>, expected: &Address) -> bool {
    values.iter().any(|value| value == *expected)
}

fn valid_pair(tokens: &Vec<Address>, token_a: &Address, token_b: &Address) -> bool {
    tokens.len() == 2 && contains(tokens, token_a) && contains(tokens, token_b)
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
        payable: vested_unclaimed.min(custody),
        unclaimed,
        vested,
    }
}

fn validate_manager(e: &Env, config: &Config) {
    let client = BackstopManagerClient::new(e, &config.manager);
    let owner = client.owner();
    if owner != config.donor {
        panic_with_error!(e, LockupBackfillError::InvalidManager);
    }
    if !contains(&client.backstops(), &config.backstop) {
        panic_with_error!(e, LockupBackfillError::InvalidBackstop);
    }
    if !contains(&client.pools(), &config.pool) {
        panic_with_error!(e, LockupBackfillError::InvalidPool);
    }

    let backstop = BackstopClient::new(e, &config.backstop);
    if backstop.backstop_token(&BackstopTier::FirstLoss, &config.pool) != config.first_lp
        || backstop.backstop_token(&BackstopTier::SecondLoss, &config.pool) != config.second_lp
    {
        panic_with_error!(e, LockupBackfillError::InvalidTierToken);
    }
    if !valid_pair(
        &CometClient::new(e, &config.first_lp).get_tokens(),
        &config.blnt,
        &config.xlm,
    ) || !valid_pair(
        &CometClient::new(e, &config.second_lp).get_tokens(),
        &config.blnt,
        &config.usdc,
    ) {
        panic_with_error!(e, LockupBackfillError::InvalidTierToken);
    }
}

#[allow(clippy::too_many_arguments)]
fn join_and_deposit(
    e: &Env,
    config: &Config,
    tier: BackstopTier,
    lp: &Address,
    pair: &Address,
    max_blnt_amount: i128,
    max_pair_amount: i128,
    lp_amount: i128,
) -> (i128, i128, i128) {
    if max_blnt_amount <= 0 || max_pair_amount <= 0 || lp_amount <= 0 {
        panic_with_error!(e, LockupBackfillError::InvalidRoute);
    }

    let tokens = CometClient::new(e, lp).get_tokens();
    if !valid_pair(&tokens, &config.blnt, pair) {
        panic_with_error!(e, LockupBackfillError::InvalidTierToken);
    }
    let mut max_amounts = Vec::new(e);
    for token in tokens.iter() {
        if token == config.blnt {
            max_amounts.push_back(max_blnt_amount);
        } else if token == *pair {
            max_amounts.push_back(max_pair_amount);
        } else {
            panic_with_error!(e, LockupBackfillError::InvalidTierToken);
        }
    }

    let contract = config.manager.clone();
    let blnt = TokenClient::new(e, &config.blnt);
    let pair_client = TokenClient::new(e, pair);
    let lp_client = TokenClient::new(e, lp);
    let blnt_before = blnt.balance(&contract);
    let pair_before = pair_client.balance(&contract);
    let lp_before = lp_client.balance(&contract);

    let manager = BackstopManagerClient::new(e, &config.manager);
    manager.c_join_pool(
        &config.donor,
        &config.backstop,
        &tier,
        &config.pool,
        &lp_amount,
        &max_amounts,
    );

    let blnt_used = blnt_before
        .checked_sub(blnt.balance(&contract))
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::BalanceMismatch));
    let pair_used = pair_before
        .checked_sub(pair_client.balance(&contract))
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::BalanceMismatch));
    if blnt_used <= 0
        || blnt_used > max_blnt_amount
        || pair_used <= 0
        || pair_used > max_pair_amount
    {
        panic_with_error!(e, LockupBackfillError::SlippageExceeded);
    }
    if lp_client.balance(&contract).checked_sub(lp_before) != Some(lp_amount) {
        panic_with_error!(e, LockupBackfillError::BalanceMismatch);
    }

    let backstop_shares = manager.b_deposit(
        &config.donor,
        &config.backstop,
        &tier,
        &config.pool,
        &lp_amount,
    );
    if backstop_shares <= 0 || lp_client.balance(&contract) != lp_before {
        panic_with_error!(e, LockupBackfillError::BalanceMismatch);
    }
    (blnt_used, pair_used, backstop_shares)
}

#[contractimpl]
impl BlntLockupBackfillContract {
    #[allow(clippy::too_many_arguments)]
    pub fn __constructor(
        e: Env,
        blnt: Address,
        source_backfill: Address,
        donor: Address,
        manager: Address,
        backstop: Address,
        pool: Address,
        first_lp: Address,
        second_lp: Address,
        xlm: Address,
        usdc: Address,
        allocation_list: Vec<(Address, i128)>,
    ) {
        let contract = e.current_contract_address();
        if allocation_list.len() != BENEFICIARY_COUNT
            || donor == contract
            || manager == contract
            || source_backfill == contract
            || blnt == first_lp
            || blnt == second_lp
            || blnt == xlm
            || blnt == usdc
            || first_lp == second_lp
            || xlm == usdc
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
        if total != TOTAL_ALLOCATION {
            panic_with_error!(&e, LockupBackfillError::AllocationTotalMismatch);
        }
        for (beneficiary, amount) in allocations::canonical(&e).iter() {
            if allocations.get(beneficiary) != Some(amount) {
                panic_with_error!(&e, LockupBackfillError::InvalidAllocation);
            }
        }

        let config = Config {
            backstop,
            blnt,
            donor,
            first_lp,
            manager,
            pool,
            second_lp,
            source_backfill,
            usdc,
            vesting_end,
            vesting_start,
            xlm,
        };
        validate_manager(&e, &config);
        storage::set_config(&e, &config);
        storage::set_allocations(&e, &allocations);
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
        storage::set_lock(&e, true);
        let contract = e.current_contract_address();
        let balance = TokenClient::new(&e, &config.blnt).balance(&contract);
        let outstanding = checked_sub(&e, TOTAL_ALLOCATION, storage::get_total_claimed(&e));
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
        let config = storage::get_config(&e);
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

    #[allow(clippy::too_many_arguments)]
    pub fn claim_source_route_and_fund(
        e: Env,
        lockup_amount: i128,
        first_lp_amount: i128,
        max_xlm_amount: i128,
        second_lp_amount: i128,
        max_usdc_amount: i128,
    ) -> RouteResult {
        storage::extend(&e);
        require_unlocked(&e);
        let config = storage::get_config(&e);
        config.donor.require_auth();
        if lockup_amount < 0 {
            panic_with_error!(&e, LockupBackfillError::InvalidAmount);
        }

        storage::set_lock(&e, true);
        validate_manager(&e, &config);
        let blnt = TokenClient::new(&e, &config.blnt);
        let contract = e.current_contract_address();
        let donor_before = blnt.balance(&config.donor);
        let contract_before = blnt.balance(&contract);
        let manager_before = blnt.balance(&config.manager);

        let source_claimed =
            SourceBackfillClient::new(&e, &config.source_backfill).claim_grant(&config.donor);
        if source_claimed <= 0
            || blnt.balance(&config.donor).checked_sub(donor_before) != Some(source_claimed)
        {
            panic_with_error!(&e, LockupBackfillError::BalanceMismatch);
        }

        let manager_budget = source_claimed / 3;
        let first_budget = manager_budget / 2;
        let second_budget = checked_sub(&e, manager_budget, first_budget);
        if first_budget <= 0 || second_budget <= 0 {
            panic_with_error!(&e, LockupBackfillError::SourceClaimTooSmall);
        }
        let post_manager = checked_sub(&e, source_claimed, manager_budget);
        if lockup_amount > post_manager {
            panic_with_error!(&e, LockupBackfillError::InvalidAmount);
        }
        let outstanding = checked_sub(&e, TOTAL_ALLOCATION, storage::get_total_claimed(&e));
        if checked_add(&e, contract_before, lockup_amount) > outstanding {
            panic_with_error!(&e, LockupBackfillError::Overfunded);
        }

        if lockup_amount > 0 {
            transfer(&e, &config.blnt, &config.donor, &contract, lockup_amount);
        }
        transfer(
            &e,
            &config.blnt,
            &config.donor,
            &config.manager,
            manager_budget,
        );

        let (first_blnt, first_xlm, first_backstop_shares) = join_and_deposit(
            &e,
            &config,
            BackstopTier::FirstLoss,
            &config.first_lp,
            &config.xlm,
            first_budget,
            max_xlm_amount,
            first_lp_amount,
        );
        let (second_blnt, second_usdc, second_backstop_shares) = join_and_deposit(
            &e,
            &config,
            BackstopTier::SecondLoss,
            &config.second_lp,
            &config.usdc,
            second_budget,
            max_usdc_amount,
            second_lp_amount,
        );
        let manager_refund =
            checked_sub(&e, manager_budget, checked_add(&e, first_blnt, second_blnt));
        if manager_refund > 0 {
            BackstopManagerClient::new(&e, &config.manager).transfer_token(
                &config.blnt,
                &config.donor,
                &manager_refund,
            );
        }

        let donor_retained = checked_add(
            &e,
            checked_sub(&e, post_manager, lockup_amount),
            manager_refund,
        );
        if blnt.balance(&contract).checked_sub(contract_before) != Some(lockup_amount)
            || blnt.balance(&config.manager) != manager_before
            || blnt.balance(&config.donor).checked_sub(donor_before) != Some(donor_retained)
        {
            panic_with_error!(&e, LockupBackfillError::BalanceMismatch);
        }
        storage::set_lock(&e, false);

        let result = RouteResult {
            donor_retained,
            first_backstop_shares,
            first_blnt,
            first_lp: first_lp_amount,
            first_xlm,
            lockup_funded: lockup_amount,
            manager_budget,
            manager_refund,
            second_backstop_shares,
            second_blnt,
            second_lp: second_lp_amount,
            second_usdc,
            source_claimed,
        };
        events::route(&e, result.clone());
        result
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
        let outstanding = checked_sub(&e, TOTAL_ALLOCATION, claimed);
        let custody = TokenClient::new(&e, &config.blnt).balance(&e.current_contract_address());
        let totals = Totals {
            allocated: TOTAL_ALLOCATION,
            claimed,
            custody,
            funding_gap: outstanding.saturating_sub(custody),
            outstanding,
        };
        storage::set_lock(&e, false);
        totals
    }
}
