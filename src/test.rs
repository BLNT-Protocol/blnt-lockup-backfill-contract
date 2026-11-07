use crate::{
    allocations::{self, ALLOCATION_0, ALLOCATION_1, ALLOCATION_2, ALLOCATION_3},
    BackstopTier, BlntLockupBackfillContract, BlntLockupBackfillContractClient, Manager,
    BENEFICIARY_COUNT, TOTAL_ALLOCATION, VESTING_DURATION_SECONDS,
};
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger},
    token::TokenClient,
    vec, Address, Env, Vec,
};

#[derive(Clone)]
#[contracttype]
enum TokenKey {
    Balance(Address),
}

#[contract]
struct MockToken;

fn token_balance(e: &Env, address: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&TokenKey::Balance(address.clone()))
        .unwrap_or(0)
}

fn set_token_balance(e: &Env, address: &Address, amount: i128) {
    e.storage()
        .persistent()
        .set(&TokenKey::Balance(address.clone()), &amount);
}

#[contractimpl]
impl MockToken {
    pub fn decimals(_e: Env) -> u32 {
        7
    }

    pub fn balance(e: Env, address: Address) -> i128 {
        token_balance(&e, &address)
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        set_token_balance(&e, &to, token_balance(&e, &to) + amount);
    }

    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        assert!(amount >= 0);
        let from_balance = token_balance(&e, &from);
        assert!(from_balance >= amount);
        set_token_balance(&e, &from, from_balance - amount);
        set_token_balance(&e, &to, token_balance(&e, &to) + amount);
    }
}

#[derive(Clone)]
#[contracttype]
enum LpKey {
    Balance(Address),
    Tokens,
}

#[contract]
struct MockLp;

fn lp_balance(e: &Env, address: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&LpKey::Balance(address.clone()))
        .unwrap_or(0)
}

fn set_lp_balance(e: &Env, address: &Address, amount: i128) {
    e.storage()
        .persistent()
        .set(&LpKey::Balance(address.clone()), &amount);
}

#[contractimpl]
impl MockLp {
    pub fn __constructor(e: Env, tokens: Vec<Address>) {
        e.storage().instance().set(&LpKey::Tokens, &tokens);
    }

    pub fn decimals(_e: Env) -> u32 {
        7
    }

    pub fn get_tokens(e: Env) -> Vec<Address> {
        e.storage().instance().get(&LpKey::Tokens).unwrap()
    }

    pub fn balance(e: Env, address: Address) -> i128 {
        lp_balance(&e, &address)
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        set_lp_balance(&e, &to, lp_balance(&e, &to) + amount);
    }

    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let from_balance = lp_balance(&e, &from);
        assert!(amount >= 0 && from_balance >= amount);
        set_lp_balance(&e, &from, from_balance - amount);
        set_lp_balance(&e, &to, lp_balance(&e, &to) + amount);
    }
}

#[derive(Clone)]
#[contracttype]
enum SourceKey {
    Blnt,
    Claim,
    End,
    Start,
}

#[contract]
struct MockSource;

#[contractimpl]
impl MockSource {
    pub fn __constructor(e: Env, blnt: Address, start: u64, end: u64, claim: i128) {
        e.storage().instance().set(&SourceKey::Blnt, &blnt);
        e.storage().instance().set(&SourceKey::Start, &start);
        e.storage().instance().set(&SourceKey::End, &end);
        e.storage().instance().set(&SourceKey::Claim, &claim);
    }

    pub fn claim_grant(e: Env, user: Address) -> i128 {
        user.require_auth();
        let claim: i128 = e.storage().instance().get(&SourceKey::Claim).unwrap();
        assert!(claim > 0);
        e.storage().instance().set(&SourceKey::Claim, &0_i128);
        let blnt: Address = e.storage().instance().get(&SourceKey::Blnt).unwrap();
        TokenClient::new(&e, &blnt).transfer(&e.current_contract_address(), &user, &claim);
        claim
    }

    pub fn get_blnt_token(e: Env) -> Address {
        e.storage().instance().get(&SourceKey::Blnt).unwrap()
    }

    pub fn get_grant_vesting_start(e: Env) -> u64 {
        e.storage().instance().get(&SourceKey::Start).unwrap()
    }

    pub fn get_grant_vesting_end(e: Env) -> u64 {
        e.storage().instance().get(&SourceKey::End).unwrap()
    }
}

#[derive(Clone)]
#[contracttype]
enum BackstopKey {
    First,
    Second,
}

#[contract]
struct MockBackstop;

#[contractimpl]
impl MockBackstop {
    pub fn __constructor(e: Env, first: Address, second: Address) {
        e.storage().instance().set(&BackstopKey::First, &first);
        e.storage().instance().set(&BackstopKey::Second, &second);
    }

    pub fn backstop_token(e: Env, tier: BackstopTier, _pool: Address) -> Address {
        match tier {
            BackstopTier::FirstLoss => e.storage().instance().get(&BackstopKey::First).unwrap(),
            BackstopTier::SecondLoss => e.storage().instance().get(&BackstopKey::Second).unwrap(),
            BackstopTier::ThirdLoss => panic!(),
        }
    }
}

#[derive(Clone)]
#[contracttype]
enum ManagerKey {
    Backstop,
    First,
    Manager,
    Owner,
    Pool,
    PoolAllowed,
    Second,
}

#[contract]
struct MockManager;

#[contractimpl]
impl MockManager {
    #[allow(clippy::too_many_arguments)]
    pub fn __constructor(
        e: Env,
        owner: Address,
        manager: Address,
        first: Address,
        second: Address,
        backstop: Address,
        pool: Address,
    ) {
        e.storage().instance().set(&ManagerKey::Owner, &owner);
        e.storage().instance().set(
            &ManagerKey::Manager,
            &Manager {
                id: manager,
                scope: 2,
            },
        );
        e.storage().instance().set(&ManagerKey::First, &first);
        e.storage().instance().set(&ManagerKey::Second, &second);
        e.storage().instance().set(&ManagerKey::Backstop, &backstop);
        e.storage().instance().set(&ManagerKey::Pool, &pool);
        e.storage().instance().set(&ManagerKey::PoolAllowed, &true);
    }

    pub fn owner(e: Env) -> Address {
        e.storage().instance().get(&ManagerKey::Owner).unwrap()
    }

    pub fn manager(e: Env) -> Manager {
        e.storage().instance().get(&ManagerKey::Manager).unwrap()
    }

    pub fn set_owner(e: Env, owner: Address) {
        Self::owner(e.clone()).require_auth();
        e.storage().instance().set(&ManagerKey::Owner, &owner);
    }

    pub fn set_manager(e: Env, manager: Address, scope: u32) {
        Self::owner(e.clone()).require_auth();
        e.storage()
            .instance()
            .set(&ManagerKey::Manager, &Manager { id: manager, scope });
    }

    pub fn set_pool_allowed(e: Env, allowed: bool) {
        Self::owner(e.clone()).require_auth();
        e.storage()
            .instance()
            .set(&ManagerKey::PoolAllowed, &allowed);
    }

    pub fn backstops(e: Env) -> Vec<Address> {
        vec![
            &e,
            e.storage().instance().get(&ManagerKey::Backstop).unwrap(),
        ]
    }

    pub fn pools(e: Env) -> Vec<Address> {
        if e.storage()
            .instance()
            .get(&ManagerKey::PoolAllowed)
            .unwrap()
        {
            vec![&e, e.storage().instance().get(&ManagerKey::Pool).unwrap()]
        } else {
            Vec::new(&e)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn c_join_pool(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        pool_amount_out: i128,
        max_amounts_in: Vec<i128>,
    ) {
        from.require_auth();
        assert_eq!(
            backstop,
            e.storage().instance().get(&ManagerKey::Backstop).unwrap()
        );
        assert_eq!(
            pool_address,
            e.storage().instance().get(&ManagerKey::Pool).unwrap()
        );
        let lp: Address = match tier {
            BackstopTier::FirstLoss => e.storage().instance().get(&ManagerKey::First).unwrap(),
            BackstopTier::SecondLoss => e.storage().instance().get(&ManagerKey::Second).unwrap(),
            BackstopTier::ThirdLoss => panic!(),
        };
        let tokens = MockLpClient::new(&e, &lp).get_tokens();
        for index in 0..tokens.len() {
            let token = tokens.get_unchecked(index);
            let amount = max_amounts_in.get_unchecked(index);
            assert!(amount > 0);
            TokenClient::new(&e, &token).transfer(&e.current_contract_address(), &lp, &amount);
        }
        MockLpClient::new(&e, &lp).mint(&e.current_contract_address(), &pool_amount_out);
    }

    pub fn b_deposit(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        amount: i128,
    ) -> i128 {
        from.require_auth();
        assert_eq!(
            pool_address,
            e.storage().instance().get(&ManagerKey::Pool).unwrap()
        );
        let lp: Address = match tier {
            BackstopTier::FirstLoss => e.storage().instance().get(&ManagerKey::First).unwrap(),
            BackstopTier::SecondLoss => e.storage().instance().get(&ManagerKey::Second).unwrap(),
            BackstopTier::ThirdLoss => panic!(),
        };
        TokenClient::new(&e, &lp).transfer(&e.current_contract_address(), &backstop, &amount);
        amount
    }
}

struct Fixture {
    e: Env,
    contract: Address,
    donor: Address,
    beneficiaries: Vec<Address>,
    blnt: Address,
    xlm: Address,
    usdc: Address,
    first_lp: Address,
    second_lp: Address,
    source: Address,
    manager: Address,
    operator: Address,
    backstop: Address,
    start: u64,
}

impl Fixture {
    fn create(source_claim: i128) -> Self {
        Self::create_with_options(source_claim, true)
    }

    fn create_without_auth(source_claim: i128) -> Self {
        Self::create_with_options(source_claim, false)
    }

    fn create_with_options(source_claim: i128, mock_auths: bool) -> Self {
        let e = Env::default();
        if mock_auths {
            e.mock_all_auths();
        }
        e.ledger().set_timestamp(2_000_000_000);

        let donor = Address::generate(&e);
        let canonical_allocations = allocations::canonical(&e);
        let mut beneficiaries = Vec::new(&e);
        for (beneficiary, _) in canonical_allocations.iter() {
            beneficiaries.push_back(beneficiary);
        }
        assert_eq!(beneficiaries.len(), BENEFICIARY_COUNT);
        let blnt = e.register(MockToken, ());
        let xlm = e.register(MockToken, ());
        let usdc = e.register(MockToken, ());
        let first_lp = e.register(MockLp, (vec![&e, blnt.clone(), xlm.clone()],));
        let second_lp = e.register(MockLp, (vec![&e, usdc.clone(), blnt.clone()],));
        let backstop = e.register(MockBackstop, (first_lp.clone(), second_lp.clone()));
        let pool = Address::generate(&e);
        let operator = Address::generate(&e);
        let manager = e.register(
            MockManager,
            (
                donor.clone(),
                operator.clone(),
                first_lp.clone(),
                second_lp.clone(),
                backstop.clone(),
                pool.clone(),
            ),
        );
        let start = e.ledger().timestamp() - 10 * 24 * 60 * 60;
        let source = e.register(
            MockSource,
            (
                blnt.clone(),
                start,
                start + VESTING_DURATION_SECONDS,
                source_claim,
            ),
        );
        MockTokenClient::new(&e, &blnt).mint(&source, &source_claim);

        assert_eq!(
            ALLOCATION_0 + ALLOCATION_1 + ALLOCATION_2 + ALLOCATION_3,
            TOTAL_ALLOCATION
        );
        let contract = e.register(
            BlntLockupBackfillContract,
            (
                blnt.clone(),
                source.clone(),
                donor.clone(),
                manager.clone(),
                backstop.clone(),
                pool,
                first_lp.clone(),
                second_lp.clone(),
                xlm.clone(),
                usdc.clone(),
                canonical_allocations,
            ),
        );

        Self {
            e,
            contract,
            donor,
            beneficiaries,
            blnt,
            xlm,
            usdc,
            first_lp,
            second_lp,
            source,
            manager,
            operator,
            backstop,
            start,
        }
    }

    fn client(&self) -> BlntLockupBackfillContractClient<'_> {
        BlntLockupBackfillContractClient::new(&self.e, &self.contract)
    }
}

#[test]
fn direct_funding_and_partial_claim_preserve_the_remainder() {
    let fixture = Fixture::create(900);
    let funder = Address::generate(&fixture.e);
    MockTokenClient::new(&fixture.e, &fixture.blnt).mint(&funder, &1_500);
    let client = fixture.client();

    assert_eq!(client.fund(&funder, &1_000), 1_000);
    let beneficiary = fixture.beneficiaries.get_unchecked(0);
    let before = client.get_claim(&beneficiary);
    assert!(before.vested > 1_000);
    assert_eq!(before.payable, 1_000);
    assert_eq!(client.claim(&beneficiary), 1_000);
    assert_eq!(client.get_claim(&beneficiary).claimed, 1_000);
    assert!(client.try_claim(&beneficiary).is_err());

    assert_eq!(client.fund(&funder, &500), 500);
    assert_eq!(client.claim(&beneficiary), 500);
    let after = client.get_claim(&beneficiary);
    assert_eq!(after.claimed, 1_500);
    assert_eq!(after.unclaimed, ALLOCATION_0 - 1_500);
}

#[test]
fn atomic_route_funds_lockups_and_deposits_both_tiers() {
    let fixture = Fixture::create(900);
    MockTokenClient::new(&fixture.e, &fixture.xlm).mint(&fixture.manager, &20);
    MockTokenClient::new(&fixture.e, &fixture.usdc).mint(&fixture.manager, &30);

    let result = fixture
        .client()
        .claim_source_route_and_fund(&200, &150, &10, &20, &150, &11, &30);
    assert_eq!(result.source_claimed, 900);
    assert_eq!(result.manager_budget, 300);
    assert_eq!(result.manager_retained, 0);
    assert!(result.manager_routed);
    assert_eq!(result.first_blnt, 150);
    assert_eq!(result.second_blnt, 150);
    assert_eq!(result.lockup_funded, 200);
    assert_eq!(result.donor_retained, 400);
    assert_eq!(result.first_lp, 10);
    assert_eq!(result.second_lp, 11);

    let blnt = MockTokenClient::new(&fixture.e, &fixture.blnt);
    assert_eq!(blnt.balance(&fixture.contract), 200);
    assert_eq!(blnt.balance(&fixture.manager), 0);
    assert_eq!(blnt.balance(&fixture.donor), 400);
    assert_eq!(
        MockLpClient::new(&fixture.e, &fixture.first_lp).balance(&fixture.backstop),
        10
    );
    assert_eq!(
        MockLpClient::new(&fixture.e, &fixture.second_lp).balance(&fixture.backstop),
        11
    );
}

#[test]
fn atomic_route_works_with_donor_owner_and_separate_high_scope_manager() {
    let fixture = Fixture::create(900);
    let manager = MockManagerClient::new(&fixture.e, &fixture.manager);
    assert_eq!(manager.owner(), fixture.donor);
    assert_eq!(manager.manager().id, fixture.operator);
    assert_eq!(manager.manager().scope, 2);

    MockTokenClient::new(&fixture.e, &fixture.xlm).mint(&fixture.manager, &20);
    MockTokenClient::new(&fixture.e, &fixture.usdc).mint(&fixture.manager, &30);
    let result = fixture
        .client()
        .claim_source_route_and_fund(&200, &150, &10, &20, &150, &11, &30);

    assert!(result.manager_routed);
    assert_eq!(result.manager_retained, 0);
    assert_eq!(result.donor_retained, 400);
    assert_eq!(
        MockLpClient::new(&fixture.e, &fixture.first_lp).balance(&fixture.backstop),
        10
    );
    assert_eq!(
        MockLpClient::new(&fixture.e, &fixture.second_lp).balance(&fixture.backstop),
        11
    );
}

#[test]
fn premature_owner_transfer_retains_the_full_one_third_in_donor() {
    let fixture = Fixture::create(900);
    let yieldblox = Address::generate(&fixture.e);
    let manager = MockManagerClient::new(&fixture.e, &fixture.manager);
    manager.set_owner(&yieldblox);
    assert_eq!(manager.manager().id, fixture.operator);

    let result = fixture
        .client()
        .claim_source_route_and_fund(&200, &0, &0, &0, &0, &0, &0);

    assert!(!result.manager_routed);
    assert_eq!(result.source_claimed, 900);
    assert_eq!(result.lockup_funded, 200);
    assert_eq!(result.manager_budget, 300);
    assert_eq!(result.manager_retained, 300);
    assert_eq!(result.donor_retained, 700);
    assert_eq!(result.first_blnt, 0);
    assert_eq!(result.second_blnt, 0);
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.donor),
        700
    );
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.manager),
        0
    );
    assert_eq!(fixture.client().get_totals().custody, 200);
}

#[test]
fn removed_manager_allowlist_access_retains_the_full_one_third_in_donor() {
    let fixture = Fixture::create(900);
    let manager = MockManagerClient::new(&fixture.e, &fixture.manager);
    manager.set_pool_allowed(&false);

    let result = fixture
        .client()
        .claim_source_route_and_fund(&200, &0, &0, &0, &0, &0, &0);

    assert!(!result.manager_routed);
    assert_eq!(result.manager_retained, 300);
    assert_eq!(result.donor_retained, 700);
    assert_eq!(fixture.client().get_totals().custody, 200);
}

#[test]
fn proportional_join_rounding_never_enters_manager_custody() {
    let fixture = Fixture::create(905);
    MockTokenClient::new(&fixture.e, &fixture.xlm).mint(&fixture.manager, &20);
    MockTokenClient::new(&fixture.e, &fixture.usdc).mint(&fixture.manager, &30);

    let result = fixture
        .client()
        .claim_source_route_and_fund(&200, &149, &10, &20, &150, &11, &30);
    assert_eq!(result.manager_budget, 301);
    assert_eq!(result.first_blnt, 149);
    assert_eq!(result.second_blnt, 150);
    assert_eq!(result.manager_retained, 2);
    assert!(result.manager_routed);
    assert_eq!(result.donor_retained, 406);
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.donor),
        406
    );
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.manager),
        0
    );
}

#[test]
fn route_failure_rolls_back_source_claim_and_first_tier() {
    let fixture = Fixture::create(900);
    MockTokenClient::new(&fixture.e, &fixture.xlm).mint(&fixture.manager, &20);
    MockTokenClient::new(&fixture.e, &fixture.usdc).mint(&fixture.manager, &30);
    let client = fixture.client();

    assert!(client
        .try_claim_source_route_and_fund(&200, &150, &10, &20, &150, &0, &30)
        .is_err());
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.donor),
        0
    );
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.source),
        900
    );
    assert_eq!(
        MockLpClient::new(&fixture.e, &fixture.first_lp).balance(&fixture.backstop),
        0
    );
    assert_eq!(client.get_totals().custody, 0);
}

#[test]
fn vesting_uses_the_source_epoch() {
    let fixture = Fixture::create(900);
    let client = fixture.client();
    let beneficiary = fixture.beneficiaries.get_unchecked(1);
    let expected =
        ALLOCATION_1 * i128::from(10 * 24 * 60 * 60) / i128::from(VESTING_DURATION_SECONDS);
    assert_eq!(client.get_claim(&beneficiary).vested, expected);

    fixture
        .e
        .ledger()
        .set_timestamp(fixture.start + VESTING_DURATION_SECONDS);
    assert_eq!(client.get_claim(&beneficiary).vested, ALLOCATION_1);
}

#[test]
fn grant_vesting_boundaries_and_flooring_are_exact() {
    let fixture = Fixture::create(900);
    let client = fixture.client();
    let beneficiary = fixture.beneficiaries.get_unchecked(2);

    fixture.e.ledger().set_timestamp(fixture.start);
    assert_eq!(client.get_claim(&beneficiary).vested, 0);

    fixture
        .e
        .ledger()
        .set_timestamp(fixture.start + VESTING_DURATION_SECONDS / 2);
    assert_eq!(client.get_claim(&beneficiary).vested, ALLOCATION_2 / 2);

    fixture
        .e
        .ledger()
        .set_timestamp(fixture.start + VESTING_DURATION_SECONDS - 1);
    assert_eq!(
        client.get_claim(&beneficiary).vested,
        ALLOCATION_2 * i128::from(VESTING_DURATION_SECONDS - 1)
            / i128::from(VESTING_DURATION_SECONDS)
    );

    fixture
        .e
        .ledger()
        .set_timestamp(fixture.start + VESTING_DURATION_SECONDS);
    assert_eq!(client.get_claim(&beneficiary).vested, ALLOCATION_2);
}

#[test]
fn authorization_is_required_for_every_mutating_entrypoint() {
    let fixture = Fixture::create_without_auth(900);
    let funder = Address::generate(&fixture.e);
    let beneficiary = fixture.beneficiaries.get_unchecked(0);
    MockTokenClient::new(&fixture.e, &fixture.blnt).mint(&funder, &100);

    assert!(fixture.client().try_fund(&funder, &100).is_err());
    assert!(fixture.client().try_claim(&beneficiary).is_err());
    assert!(fixture
        .client()
        .try_claim_source_route_and_fund(&100, &150, &10, &20, &150, &10, &20)
        .is_err());
    assert_eq!(fixture.client().get_totals().custody, 0);
}

#[test]
fn unknown_beneficiaries_and_overfunding_are_rejected() {
    let fixture = Fixture::create(900);
    let funder = Address::generate(&fixture.e);
    let stranger = Address::generate(&fixture.e);
    let token = MockTokenClient::new(&fixture.e, &fixture.blnt);
    token.mint(&funder, &(TOTAL_ALLOCATION + 1));

    assert!(fixture.client().try_get_claim(&stranger).is_err());
    assert!(fixture.client().try_claim(&stranger).is_err());
    assert!(fixture
        .client()
        .try_fund(&funder, &(TOTAL_ALLOCATION + 1))
        .is_err());

    assert_eq!(
        fixture.client().fund(&funder, &TOTAL_ALLOCATION),
        TOTAL_ALLOCATION
    );
    assert!(fixture.client().try_fund(&funder, &1).is_err());
    assert_eq!(fixture.client().get_totals().funding_gap, 0);
}

#[test]
#[should_panic]
fn constructor_rejects_same_total_with_noncanonical_allocations() {
    let fixture = Fixture::create(900);
    let config = fixture.client().get_config();
    let allocations = vec![
        &fixture.e,
        (fixture.beneficiaries.get_unchecked(0), ALLOCATION_1),
        (fixture.beneficiaries.get_unchecked(1), ALLOCATION_0),
        (fixture.beneficiaries.get_unchecked(2), ALLOCATION_2),
        (fixture.beneficiaries.get_unchecked(3), ALLOCATION_3),
    ];

    fixture.e.register(
        BlntLockupBackfillContract,
        (
            config.blnt,
            config.source_backfill,
            config.donor,
            config.manager,
            config.backstop,
            config.pool,
            config.first_lp,
            config.second_lp,
            config.xlm,
            config.usdc,
            allocations,
        ),
    );
}

#[test]
fn first_claimant_cannot_erase_a_later_claimants_entitlement() {
    let fixture = Fixture::create(900);
    fixture
        .e
        .ledger()
        .set_timestamp(fixture.start + VESTING_DURATION_SECONDS);
    let funder = Address::generate(&fixture.e);
    let token = MockTokenClient::new(&fixture.e, &fixture.blnt);
    token.mint(&funder, &150);
    let client = fixture.client();
    client.fund(&funder, &100);

    let first = fixture.beneficiaries.get_unchecked(0);
    let second = fixture.beneficiaries.get_unchecked(1);
    assert_eq!(client.claim(&first), 100);
    let waiting = client.get_claim(&second);
    assert_eq!(waiting.claimed, 0);
    assert_eq!(waiting.unclaimed, ALLOCATION_1);
    assert_eq!(waiting.payable, 0);

    client.fund(&funder, &50);
    assert_eq!(client.claim(&second), 50);
    assert_eq!(client.get_claim(&second).unclaimed, ALLOCATION_1 - 50);
}

#[test]
fn full_funding_and_distribution_leave_no_outstanding_claims() {
    let fixture = Fixture::create(900);
    fixture
        .e
        .ledger()
        .set_timestamp(fixture.start + VESTING_DURATION_SECONDS);
    let funder = Address::generate(&fixture.e);
    MockTokenClient::new(&fixture.e, &fixture.blnt).mint(&funder, &TOTAL_ALLOCATION);
    let client = fixture.client();
    client.fund(&funder, &TOTAL_ALLOCATION);

    let mut distributed = 0_i128;
    for beneficiary in fixture.beneficiaries.iter() {
        distributed += client.claim(&beneficiary);
        let state = client.get_claim(&beneficiary);
        assert_eq!(state.claimed, state.allocation);
        assert_eq!(state.unclaimed, 0);
        assert_eq!(state.payable, 0);
    }
    assert_eq!(distributed, TOTAL_ALLOCATION);
    assert_eq!(
        client.get_totals(),
        crate::Totals {
            allocated: TOTAL_ALLOCATION,
            claimed: TOTAL_ALLOCATION,
            custody: 0,
            funding_gap: 0,
            outstanding: 0,
        }
    );
}

#[test]
fn either_counterasset_shortfall_rolls_back_the_entire_route() {
    let first = Fixture::create(900);
    MockTokenClient::new(&first.e, &first.xlm).mint(&first.manager, &19);
    MockTokenClient::new(&first.e, &first.usdc).mint(&first.manager, &30);
    assert!(first
        .client()
        .try_claim_source_route_and_fund(&200, &150, &10, &20, &150, &11, &30)
        .is_err());
    assert_eq!(
        MockTokenClient::new(&first.e, &first.blnt).balance(&first.source),
        900
    );
    assert_eq!(first.client().get_totals().custody, 0);

    let second = Fixture::create(900);
    MockTokenClient::new(&second.e, &second.xlm).mint(&second.manager, &20);
    MockTokenClient::new(&second.e, &second.usdc).mint(&second.manager, &29);
    assert!(second
        .client()
        .try_claim_source_route_and_fund(&200, &150, &10, &20, &150, &11, &30)
        .is_err());
    assert_eq!(
        MockTokenClient::new(&second.e, &second.blnt).balance(&second.source),
        900
    );
    assert_eq!(
        MockLpClient::new(&second.e, &second.first_lp).balance(&second.backstop),
        0
    );
    assert_eq!(second.client().get_totals().custody, 0);
}

#[test]
fn route_cannot_overfund_replacement_claims_but_allows_zero_top_up() {
    let fixture = Fixture::create(900);
    let funder = Address::generate(&fixture.e);
    MockTokenClient::new(&fixture.e, &fixture.blnt).mint(&funder, &TOTAL_ALLOCATION);
    fixture.client().fund(&funder, &TOTAL_ALLOCATION);
    MockTokenClient::new(&fixture.e, &fixture.xlm).mint(&fixture.manager, &20);
    MockTokenClient::new(&fixture.e, &fixture.usdc).mint(&fixture.manager, &30);

    assert!(fixture
        .client()
        .try_claim_source_route_and_fund(&1, &150, &10, &20, &150, &11, &30)
        .is_err());
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.source),
        900
    );
    assert_eq!(fixture.client().get_totals().custody, TOTAL_ALLOCATION);

    let result = fixture
        .client()
        .claim_source_route_and_fund(&0, &150, &10, &20, &150, &11, &30);
    assert_eq!(result.lockup_funded, 0);
    assert_eq!(result.donor_retained, 600);
    assert_eq!(fixture.client().get_totals().custody, TOTAL_ALLOCATION);
}
