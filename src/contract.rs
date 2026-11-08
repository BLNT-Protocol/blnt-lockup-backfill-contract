use crate::{
    allocations,
    clients::{BackstopClient, BackstopManagerClient, CometClient, SourceBackfillClient},
    errors::LockupBackfillError,
    events, storage,
    types::{BackstopTier, ClaimState, Config, GrantFundingDetails, GrantFundingResult, Totals},
    BENEFICIARY_COUNT, TOTAL_ALLOCATION, VESTING_DURATION_SECONDS,
};
use soroban_sdk::{
    contract, contractimpl, panic_with_error, token::TokenClient, Address, Env, Map, Vec, I256,
};

const COMET_SCALE: i128 = 1_000_000_000_000_000_000;
const BPS_SCALE: i128 = 10_000;
const JOIN_SLIPPAGE_BPS: i128 = 100;

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

fn mul_div_floor(e: &Env, value: i128, numerator: i128, denominator: i128) -> i128 {
    if value < 0 || numerator < 0 || denominator <= 0 {
        panic_with_error!(e, LockupBackfillError::InvalidRoute);
    }
    I256::from_i128(e, value)
        .mul(&I256::from_i128(e, numerator))
        .div(&I256::from_i128(e, denominator))
        .to_i128()
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::Overflow))
}

fn mul_div_ceil(e: &Env, value: i128, numerator: i128, denominator: i128) -> i128 {
    if value < 0 || numerator < 0 || denominator <= 0 {
        panic_with_error!(e, LockupBackfillError::InvalidRoute);
    }
    let product = I256::from_i128(e, value).mul(&I256::from_i128(e, numerator));
    let divisor = I256::from_i128(e, denominator);
    let quotient = product.div(&divisor);
    let rounded = if product.rem_euclid(&divisor) == I256::from_i32(e, 0) {
        quotient
    } else {
        quotient
            .checked_add(&I256::from_i32(e, 1))
            .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::Overflow))
    };
    rounded
        .to_i128()
        .unwrap_or_else(|| panic_with_error!(e, LockupBackfillError::Overflow))
}

fn require_claims_open(e: &Env) {
    if storage::claims_closed(e) {
        panic_with_error!(e, LockupBackfillError::ClaimsClosed);
    }
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
        payable: if !storage::claims_closed(e) {
            vested_unclaimed.min(custody)
        } else {
            0
        },
        unclaimed,
        vested,
    }
}

fn validate_constructor_bindings(e: &Env, config: &Config) -> (bool, bool) {
    let backstop = BackstopClient::new(e, &config.backstop);
    if backstop.backstop_token(&BackstopTier::FirstLoss, &config.pool) != config.first_lp
        || backstop.backstop_token(&BackstopTier::SecondLoss, &config.pool) != config.second_lp
    {
        panic_with_error!(e, LockupBackfillError::InvalidTierToken);
    }
    let first_tokens = CometClient::new(e, &config.first_lp).get_tokens();
    let second_tokens = CometClient::new(e, &config.second_lp).get_tokens();
    if !valid_pair(&first_tokens, &config.blnt, &config.xlm)
        || !valid_pair(&second_tokens, &config.blnt, &config.usdc)
    {
        panic_with_error!(e, LockupBackfillError::InvalidTierToken);
    }
    (
        first_tokens.get_unchecked(0) == config.blnt,
        second_tokens.get_unchecked(0) == config.blnt,
    )
}

fn validate_manager(e: &Env, config: &Config) {
    let client = BackstopManagerClient::new(e, &config.manager);
    if client.owner() != config.owner {
        panic_with_error!(e, LockupBackfillError::InvalidManager);
    }
    if !contains(&client.backstops(), &config.backstop) {
        panic_with_error!(e, LockupBackfillError::InvalidBackstop);
    }
    if !contains(&client.pools(), &config.pool) {
        panic_with_error!(e, LockupBackfillError::InvalidPool);
    }
}

struct JoinQuote {
    blnt: i128,
    lp: i128,
    pair: i128,
    max_pair: i128,
}

fn quote_join(
    e: &Env,
    config: &Config,
    lp: &Address,
    pair: &Address,
    blnt_budget: i128,
) -> JoinQuote {
    if blnt_budget <= 0 {
        panic_with_error!(e, LockupBackfillError::InvalidRoute);
    }
    let client = CometClient::new(e, lp);
    let supply = client.get_total_supply();
    let blnt_reserve = client.get_balance(&config.blnt);
    let pair_reserve = client.get_balance(pair);
    if supply <= 0 || blnt_reserve <= 0 || pair_reserve <= 0 {
        panic_with_error!(e, LockupBackfillError::InvalidRoute);
    }

    // Comet rounds the join ratio and both proportional inputs up. Compute the
    // greatest LP output whose rounded BLNT input cannot exceed this tier's
    // immutable share of the claimed grant.
    let max_ratio = mul_div_floor(e, blnt_budget, COMET_SCALE, blnt_reserve);
    let lp_amount = mul_div_floor(e, max_ratio, supply, COMET_SCALE);
    if lp_amount <= 0 {
        panic_with_error!(e, LockupBackfillError::InvalidRoute);
    }
    let join_ratio = mul_div_ceil(e, lp_amount, COMET_SCALE, supply);
    let blnt_amount = mul_div_ceil(e, blnt_reserve, join_ratio, COMET_SCALE);
    let pair_amount = mul_div_ceil(e, pair_reserve, join_ratio, COMET_SCALE);
    if blnt_amount <= 0 || blnt_amount > blnt_budget || pair_amount <= 0 {
        panic_with_error!(e, LockupBackfillError::InvalidRoute);
    }
    let max_pair = mul_div_ceil(e, pair_amount, BPS_SCALE + JOIN_SLIPPAGE_BPS, BPS_SCALE);
    JoinQuote {
        blnt: blnt_amount,
        lp: lp_amount,
        pair: pair_amount,
        max_pair,
    }
}

#[allow(clippy::too_many_arguments)]
fn join_and_deposit(
    e: &Env,
    config: &Config,
    tier: BackstopTier,
    lp: &Address,
    blnt_first: bool,
    quote: &JoinQuote,
) -> (i128, i128, i128) {
    let mut max_amounts = Vec::new(e);
    if blnt_first {
        max_amounts.push_back(quote.blnt);
        max_amounts.push_back(quote.max_pair);
    } else {
        max_amounts.push_back(quote.max_pair);
        max_amounts.push_back(quote.blnt);
    }

    let contract = config.manager.clone();
    let lp_client = TokenClient::new(e, lp);
    let lp_before = lp_client.balance(&contract);

    let manager = BackstopManagerClient::new(e, &config.manager);
    manager.c_join_pool(
        &config.owner,
        &config.backstop,
        &tier,
        &config.pool,
        &quote.lp,
        &max_amounts,
    );

    let backstop_shares = manager.b_deposit(
        &config.owner,
        &config.backstop,
        &tier,
        &config.pool,
        &quote.lp,
    );
    if lp_client.balance(&contract) != lp_before {
        panic_with_error!(e, LockupBackfillError::BalanceMismatch);
    }
    (quote.blnt, quote.pair, backstop_shares)
}

#[contractimpl]
impl BlntLockupBackfillContract {
    #[allow(clippy::too_many_arguments)]
    pub fn __constructor(
        e: Env,
        blnt: Address,
        source_backfill: Address,
        owner: Address,
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
            || owner == contract
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

        let mut config = Config {
            backstop,
            blnt,
            first_blnt_first: false,
            owner,
            first_lp,
            manager,
            pool,
            second_blnt_first: false,
            second_lp,
            source_backfill,
            usdc,
            vesting_end,
            vesting_start,
            xlm,
        };
        let (first_blnt_first, second_blnt_first) = validate_constructor_bindings(&e, &config);
        config.first_blnt_first = first_blnt_first;
        config.second_blnt_first = second_blnt_first;
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

    pub fn claim_grant_and_fund(e: Env) -> GrantFundingResult {
        storage::extend(&e);
        require_unlocked(&e);
        let config = storage::get_config(&e);
        config.owner.require_auth();

        storage::set_lock(&e, true);
        let blnt = TokenClient::new(&e, &config.blnt);
        let contract = e.current_contract_address();
        let owner_before = blnt.balance(&config.owner);
        let contract_before = blnt.balance(&contract);
        let manager_before = blnt.balance(&config.manager);

        let source_claimed =
            SourceBackfillClient::new(&e, &config.source_backfill).claim_grant(&config.owner);

        let manager_budget = source_claimed / 3;
        let first_budget = manager_budget / 2;
        let second_budget = checked_sub(&e, manager_budget, first_budget);
        let post_manager = checked_sub(&e, source_claimed, manager_budget);
        let outstanding = checked_sub(&e, TOTAL_ALLOCATION, storage::get_total_claimed(&e));
        let funding_gap = outstanding.saturating_sub(contract_before);
        let lockup_amount = if storage::claims_closed(&e) {
            0
        } else {
            funding_gap.min(post_manager)
        };

        if lockup_amount > 0 {
            blnt.transfer(&config.owner, &contract, &lockup_amount);
        }
        let first_quote = quote_join(&e, &config, &config.first_lp, &config.xlm, first_budget);
        let second_quote = quote_join(&e, &config, &config.second_lp, &config.usdc, second_budget);
        let routed = checked_add(&e, first_quote.blnt, second_quote.blnt);
        blnt.transfer(&config.owner, &config.manager, &routed);

        let (first_blnt, first_xlm, first_backstop_shares) = join_and_deposit(
            &e,
            &config,
            BackstopTier::FirstLoss,
            &config.first_lp,
            config.first_blnt_first,
            &first_quote,
        );
        let (second_blnt, second_usdc, second_backstop_shares) = join_and_deposit(
            &e,
            &config,
            BackstopTier::SecondLoss,
            &config.second_lp,
            config.second_blnt_first,
            &second_quote,
        );
        let first_lp = first_quote.lp;
        let second_lp = second_quote.lp;
        let routed = checked_add(&e, first_blnt, second_blnt);
        let manager_retained = checked_sub(&e, manager_budget, routed);
        let owner_retained =
            checked_sub(&e, checked_sub(&e, source_claimed, lockup_amount), routed);
        if blnt.balance(&contract).checked_sub(contract_before) != Some(lockup_amount)
            || blnt.balance(&config.manager) != manager_before
            || blnt.balance(&config.owner).checked_sub(owner_before) != Some(owner_retained)
        {
            panic_with_error!(&e, LockupBackfillError::BalanceMismatch);
        }
        storage::set_lock(&e, false);

        let result = GrantFundingResult {
            backstop_funded: true,
            owner_retained,
            grant_claimed: source_claimed,
            lockup_funded: lockup_amount,
        };
        events::grant_funding(
            &e,
            GrantFundingDetails {
                backstop_budget: manager_budget,
                backstop_funded: true,
                backstop_unspent: manager_retained,
                owner_retained,
                first_backstop_shares,
                first_blnt,
                first_lp,
                first_xlm,
                grant_claimed: source_claimed,
                lockup_funded: lockup_amount,
                second_backstop_shares,
                second_blnt,
                second_lp,
                second_usdc,
            },
        );
        result
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
