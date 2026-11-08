use crate::{
    BlntLockupBackfillContract, BlntLockupBackfillContractClient, Config, VESTING_DURATION_SECONDS,
};
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token::TokenClient,
    vec, Address, Env, IntoVal, Vec,
};

const ALLOCATION_0: i128 = 8_892_037_498_416;
const ALLOCATION_1: i128 = 7_597_459_398_743;
const ALLOCATION_2: i128 = 7_600_947_904_368;
const ALLOCATION_3: i128 = 13_351_961_001_616;
const TOTAL_ALLOCATION: i128 = ALLOCATION_0 + ALLOCATION_1 + ALLOCATION_2 + ALLOCATION_3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
#[allow(clippy::enum_variant_names)]
enum BackstopTier {
    FirstLoss,
    SecondLoss,
    ThirdLoss,
}

#[derive(Clone)]
#[contracttype]
enum TokenKey {
    Allowance(Address, Address),
    Balance(Address),
}

#[derive(Clone)]
#[contracttype]
struct MockAllowance {
    amount: i128,
    expiration_ledger: u32,
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

fn token_allowance(e: &Env, from: &Address, spender: &Address) -> i128 {
    let allowance = e
        .storage()
        .persistent()
        .get::<_, MockAllowance>(&TokenKey::Allowance(from.clone(), spender.clone()));
    match allowance {
        Some(value) if value.expiration_ledger >= e.ledger().sequence() => value.amount,
        _ => 0,
    }
}

fn move_token(e: &Env, from: &Address, to: &Address, amount: i128) {
    assert!(amount >= 0);
    let from_balance = token_balance(e, from);
    assert!(from_balance >= amount);
    set_token_balance(e, from, from_balance - amount);
    set_token_balance(e, to, token_balance(e, to) + amount);
}

#[contractimpl]
impl MockToken {
    pub fn decimals(_e: Env) -> u32 {
        7
    }

    pub fn balance(e: Env, address: Address) -> i128 {
        token_balance(&e, &address)
    }

    pub fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        token_allowance(&e, &from, &spender)
    }

    pub fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        from.require_auth();
        assert!(amount >= 0);
        assert!(amount == 0 || expiration_ledger >= e.ledger().sequence());
        e.storage().persistent().set(
            &TokenKey::Allowance(from, spender),
            &MockAllowance {
                amount,
                expiration_ledger,
            },
        );
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        set_token_balance(&e, &to, token_balance(&e, &to) + amount);
    }

    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        move_token(&e, &from, &to, amount);
    }

    pub fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        assert!(amount >= 0);
        let allowance = token_allowance(&e, &from, &spender);
        assert!(allowance >= amount);
        e.storage().persistent().set(
            &TokenKey::Allowance(from.clone(), spender.clone()),
            &MockAllowance {
                amount: allowance - amount,
                expiration_ledger: e
                    .storage()
                    .persistent()
                    .get::<_, MockAllowance>(&TokenKey::Allowance(from.clone(), spender))
                    .unwrap()
                    .expiration_ledger,
            },
        );
        move_token(&e, &from, &to, amount);
    }
}

#[derive(Clone)]
#[contracttype]
enum LpKey {
    Balance(Address),
    TotalSupply,
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
    pub fn __constructor(e: Env, tokens: Vec<Address>, total_supply: i128) {
        e.storage().instance().set(&LpKey::Tokens, &tokens);
        e.storage()
            .instance()
            .set(&LpKey::TotalSupply, &total_supply);
    }

    pub fn decimals(_e: Env) -> u32 {
        7
    }

    pub fn get_tokens(e: Env) -> Vec<Address> {
        e.storage().instance().get(&LpKey::Tokens).unwrap()
    }

    pub fn get_balance(e: Env, token: Address) -> i128 {
        TokenClient::new(&e, &token).balance(&e.current_contract_address())
    }

    pub fn get_total_supply(e: Env) -> i128 {
        e.storage().instance().get(&LpKey::TotalSupply).unwrap()
    }

    pub fn balance(e: Env, address: Address) -> i128 {
        lp_balance(&e, &address)
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        set_lp_balance(&e, &to, lp_balance(&e, &to) + amount);
        let supply: i128 = e.storage().instance().get(&LpKey::TotalSupply).unwrap();
        e.storage()
            .instance()
            .set(&LpKey::TotalSupply, &(supply + amount));
    }

    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let from_balance = lp_balance(&e, &from);
        assert!(amount >= 0 && from_balance >= amount);
        set_lp_balance(&e, &from, from_balance - amount);
        set_lp_balance(&e, &to, lp_balance(&e, &to) + amount);
    }
}

fn test_mul_div_ceil(value: i128, numerator: i128, denominator: i128) -> i128 {
    let product = value * numerator;
    product / denominator + i128::from(product % denominator != 0)
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

    pub fn set_claim(e: Env, claim: i128) {
        e.storage().instance().set(&SourceKey::Claim, &claim);
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
    FailFirstClaim,
    FailSecondClaim,
    First,
    FirstClaims,
    Manager,
    Owner,
    Pool,
    PoolAllowed,
    Second,
    SecondClaims,
}

#[derive(Clone)]
#[contracttype]
struct MockManagerRole {
    id: Address,
    scope: u32,
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
        scope: u32,
        first: Address,
        second: Address,
        backstop: Address,
        pool: Address,
    ) {
        e.storage().instance().set(&ManagerKey::Owner, &owner);
        e.storage().instance().set(
            &ManagerKey::Manager,
            &MockManagerRole { id: manager, scope },
        );
        e.storage().instance().set(&ManagerKey::First, &first);
        e.storage().instance().set(&ManagerKey::Second, &second);
        e.storage().instance().set(&ManagerKey::Backstop, &backstop);
        e.storage().instance().set(&ManagerKey::Pool, &pool);
        e.storage().instance().set(&ManagerKey::PoolAllowed, &true);
        e.storage()
            .instance()
            .set(&ManagerKey::FailFirstClaim, &false);
        e.storage()
            .instance()
            .set(&ManagerKey::FailSecondClaim, &false);
        e.storage().instance().set(&ManagerKey::FirstClaims, &0_u32);
        e.storage()
            .instance()
            .set(&ManagerKey::SecondClaims, &0_u32);
    }

    pub fn owner(e: Env) -> Address {
        e.storage().instance().get(&ManagerKey::Owner).unwrap()
    }

    pub fn manager(e: Env) -> MockManagerRole {
        e.storage().instance().get(&ManagerKey::Manager).unwrap()
    }

    pub fn set_owner(e: Env, owner: Address) {
        Self::owner(e.clone()).require_auth();
        e.storage().instance().set(&ManagerKey::Owner, &owner);
    }

    pub fn set_manager(e: Env, manager: Address, scope: u32) {
        Self::owner(e.clone()).require_auth();
        e.storage().instance().set(
            &ManagerKey::Manager,
            &MockManagerRole { id: manager, scope },
        );
    }

    pub fn set_pool_allowed(e: Env, allowed: bool) {
        Self::owner(e.clone()).require_auth();
        e.storage()
            .instance()
            .set(&ManagerKey::PoolAllowed, &allowed);
    }

    pub fn set_claim_failure(e: Env, tier: BackstopTier, fail: bool) {
        Self::owner(e.clone()).require_auth();
        let key = match tier {
            BackstopTier::FirstLoss => ManagerKey::FailFirstClaim,
            BackstopTier::SecondLoss => ManagerKey::FailSecondClaim,
            BackstopTier::ThirdLoss => panic!(),
        };
        e.storage().instance().set(&key, &fail);
    }

    pub fn emission_claims(e: Env, tier: BackstopTier) -> u32 {
        let key = match tier {
            BackstopTier::FirstLoss => ManagerKey::FirstClaims,
            BackstopTier::SecondLoss => ManagerKey::SecondClaims,
            BackstopTier::ThirdLoss => panic!(),
        };
        e.storage().instance().get(&key).unwrap()
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
        let owner: Address = e.storage().instance().get(&ManagerKey::Owner).unwrap();
        let manager: MockManagerRole = e.storage().instance().get(&ManagerKey::Manager).unwrap();
        assert!(from == owner || (from == manager.id && manager.scope >= 2));
        assert_eq!(
            backstop,
            e.storage().instance().get(&ManagerKey::Backstop).unwrap()
        );
        assert_eq!(
            pool_address,
            e.storage().instance().get(&ManagerKey::Pool).unwrap()
        );
        assert!(e
            .storage()
            .instance()
            .get::<_, bool>(&ManagerKey::PoolAllowed)
            .unwrap());
        let lp: Address = match tier {
            BackstopTier::FirstLoss => e.storage().instance().get(&ManagerKey::First).unwrap(),
            BackstopTier::SecondLoss => e.storage().instance().get(&ManagerKey::Second).unwrap(),
            BackstopTier::ThirdLoss => panic!(),
        };
        let lp_client = MockLpClient::new(&e, &lp);
        let tokens = lp_client.get_tokens();
        let supply = lp_client.get_total_supply();
        let join_ratio = test_mul_div_ceil(pool_amount_out, 1_000_000_000_000_000_000, supply);
        let mut exact_limits = 0;
        let mut slippage_limits = 0;
        for index in 0..tokens.len() {
            let token = tokens.get_unchecked(index);
            let reserve = lp_client.get_balance(&token);
            let amount = test_mul_div_ceil(reserve, join_ratio, 1_000_000_000_000_000_000);
            let maximum = max_amounts_in.get_unchecked(index);
            assert!(amount > 0 && amount <= maximum);
            if maximum == amount {
                exact_limits += 1;
            } else {
                assert_eq!(maximum, test_mul_div_ceil(amount, 10_100, 10_000));
                slippage_limits += 1;
            }
            TokenClient::new(&e, &token).transfer(&e.current_contract_address(), &lp, &amount);
        }
        assert_eq!(exact_limits, 1);
        assert_eq!(slippage_limits, 1);
        lp_client.mint(&e.current_contract_address(), &pool_amount_out);
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
        let owner: Address = e.storage().instance().get(&ManagerKey::Owner).unwrap();
        let manager: MockManagerRole = e.storage().instance().get(&ManagerKey::Manager).unwrap();
        assert!(from == owner || (from == manager.id && manager.scope >= 1));
        assert_eq!(
            backstop,
            e.storage().instance().get(&ManagerKey::Backstop).unwrap()
        );
        assert_eq!(
            pool_address,
            e.storage().instance().get(&ManagerKey::Pool).unwrap()
        );
        assert!(e
            .storage()
            .instance()
            .get::<_, bool>(&ManagerKey::PoolAllowed)
            .unwrap());
        let lp: Address = match tier {
            BackstopTier::FirstLoss => e.storage().instance().get(&ManagerKey::First).unwrap(),
            BackstopTier::SecondLoss => e.storage().instance().get(&ManagerKey::Second).unwrap(),
            BackstopTier::ThirdLoss => panic!(),
        };
        TokenClient::new(&e, &lp).transfer(&e.current_contract_address(), &backstop, &amount);
        amount
    }

    pub fn b_claim(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        min_lp_tokens_out: i128,
    ) -> i128 {
        from.require_auth();
        let owner: Address = e.storage().instance().get(&ManagerKey::Owner).unwrap();
        let manager: MockManagerRole = e.storage().instance().get(&ManagerKey::Manager).unwrap();
        assert!(from == owner || from == manager.id);
        assert_eq!(
            backstop,
            e.storage().instance().get(&ManagerKey::Backstop).unwrap()
        );
        assert_eq!(
            pool_address,
            e.storage().instance().get(&ManagerKey::Pool).unwrap()
        );
        assert!(e
            .storage()
            .instance()
            .get::<_, bool>(&ManagerKey::PoolAllowed)
            .unwrap());
        assert_eq!(min_lp_tokens_out, 0);
        let (fail_key, claims_key) = match tier {
            BackstopTier::FirstLoss => (ManagerKey::FailFirstClaim, ManagerKey::FirstClaims),
            BackstopTier::SecondLoss => (ManagerKey::FailSecondClaim, ManagerKey::SecondClaims),
            BackstopTier::ThirdLoss => panic!(),
        };
        assert!(!e.storage().instance().get::<_, bool>(&fail_key).unwrap());
        let claims: u32 = e.storage().instance().get(&claims_key).unwrap();
        e.storage().instance().set(&claims_key, &(claims + 1));
        0
    }
}

struct Fixture {
    e: Env,
    contract: Address,
    owner: Address,
    beneficiaries: Vec<Address>,
    blnt: Address,
    source: Address,
    manager: Address,
    operator: Address,
    start: u64,
}

impl Fixture {
    fn create(source_claim: i128) -> Self {
        Self::create_with_options(source_claim, true)
    }

    fn create_without_manager_access(source_claim: i128) -> Self {
        Self::create_with_manager_access(source_claim, true, false, false, 2)
    }

    fn create_as_nonowner_high_scope_manager(source_claim: i128) -> Self {
        Self::create_with_manager_access(source_claim, true, false, true, 2)
    }

    fn create_as_owner_with_manager_scope(source_claim: i128, scope: u32) -> Self {
        Self::create_with_manager_access(source_claim, true, true, false, scope)
    }

    fn create_without_auth(source_claim: i128) -> Self {
        Self::create_with_options(source_claim, false)
    }

    fn create_with_options(source_claim: i128, mock_auths: bool) -> Self {
        Self::create_with_manager_access(source_claim, mock_auths, true, false, 2)
    }

    fn create_with_manager_access(
        source_claim: i128,
        mock_auths: bool,
        owner_owns_manager: bool,
        owner_is_manager: bool,
        manager_scope: u32,
    ) -> Self {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        if mock_auths {
            e.mock_all_auths();
        }
        e.ledger().set_timestamp(2_000_000_000);

        let owner = Address::generate(&e);
        let beneficiaries = vec![
            &e,
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
            Address::generate(&e),
        ];
        let allocation_list = vec![
            &e,
            (beneficiaries.get_unchecked(0), ALLOCATION_0),
            (beneficiaries.get_unchecked(1), ALLOCATION_1),
            (beneficiaries.get_unchecked(2), ALLOCATION_2),
            (beneficiaries.get_unchecked(3), ALLOCATION_3),
        ];
        let blnt = e.register(MockToken, ());
        let xlm = e.register(MockToken, ());
        let usdc = e.register(MockToken, ());
        let first_lp = e.register(MockLp, (vec![&e, blnt.clone(), xlm.clone()], 100_i128));
        let second_lp = e.register(MockLp, (vec![&e, usdc.clone(), blnt.clone()], 110_i128));
        MockTokenClient::new(&e, &blnt).mint(&first_lp, &1_500);
        MockTokenClient::new(&e, &xlm).mint(&first_lp, &200);
        MockTokenClient::new(&e, &blnt).mint(&second_lp, &1_500);
        MockTokenClient::new(&e, &usdc).mint(&second_lp, &300);
        let backstop = e.register(MockBackstop, (first_lp.clone(), second_lp.clone()));
        let pool = Address::generate(&e);
        let manager_owner = if owner_owns_manager {
            owner.clone()
        } else {
            Address::generate(&e)
        };
        let operator = if owner_is_manager {
            owner.clone()
        } else {
            Address::generate(&e)
        };
        let manager = e.register(
            MockManager,
            (
                manager_owner,
                operator.clone(),
                manager_scope,
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

        let contract = e.register(
            BlntLockupBackfillContract,
            (
                blnt.clone(),
                source.clone(),
                owner.clone(),
                manager.clone(),
                allocation_list,
            ),
        );
        if !mock_auths {
            e.mock_all_auths();
        }
        TokenClient::new(&e, &blnt).approve(&owner, &contract, &i128::MAX, &u32::MAX);
        if !mock_auths {
            e.set_auths(&[]);
        }

        Self {
            e,
            contract,
            owner,
            beneficiaries,
            blnt,
            source,
            manager,
            operator,
            start,
        }
    }

    fn client(&self) -> BlntLockupBackfillContractClient<'_> {
        BlntLockupBackfillContractClient::new(&self.e, &self.contract)
    }
}

fn register_with_allocations(
    e: &Env,
    config: Config,
    allocations: Vec<(Address, i128)>,
) -> Address {
    e.register(
        BlntLockupBackfillContract,
        (
            config.blnt,
            config.source_backfill,
            config.owner,
            config.manager,
            allocations,
        ),
    )
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
fn claim_and_fund_splits_blnt_between_lockups_and_manager() {
    let fixture = Fixture::create(900);

    let result = fixture.client().claim_and_fund();
    assert_eq!(result, 900);

    let blnt = MockTokenClient::new(&fixture.e, &fixture.blnt);
    assert_eq!(blnt.balance(&fixture.contract), 600);
    assert_eq!(blnt.balance(&fixture.manager), 300);
    assert_eq!(blnt.balance(&fixture.owner), 0);
    assert_eq!(blnt.balance(&fixture.source), 0);
}

#[test]
fn claim_and_fund_auth_remains_valid_when_the_grant_changes_after_preparation() {
    let fixture = Fixture::create(900);
    fixture.e.set_auths(&[]);

    let source_invocation = MockAuthInvoke {
        contract: &fixture.source,
        fn_name: "claim_grant",
        args: vec![&fixture.e, fixture.owner.clone().into_val(&fixture.e)],
        sub_invokes: &[],
    };
    let source_invocations = [source_invocation];
    let prepared_owner_invocation = MockAuthInvoke {
        contract: &fixture.contract,
        fn_name: "claim_and_fund",
        args: vec![&fixture.e],
        sub_invokes: &source_invocations,
    };

    MockSourceClient::new(&fixture.e, &fixture.source).set_claim(&905);
    MockTokenClient::new(&fixture.e, &fixture.blnt).mint(&fixture.source, &5);
    fixture
        .e
        .ledger()
        .set_timestamp(fixture.e.ledger().timestamp() + 5);

    let result = fixture
        .client()
        .mock_auths(&[MockAuth {
            address: &fixture.owner,
            invoke: &prepared_owner_invocation,
        }])
        .claim_and_fund();

    assert_eq!(result, 905);
    let blnt = MockTokenClient::new(&fixture.e, &fixture.blnt);
    assert_eq!(blnt.balance(&fixture.contract), 604);
    assert_eq!(blnt.balance(&fixture.manager), 301);
    assert_eq!(
        blnt.allowance(&fixture.owner, &fixture.contract),
        i128::MAX - 905
    );
}

#[test]
fn insufficient_allowance_rolls_back_the_source_claim_and_transfers() {
    let fixture = Fixture::create(900);
    let blnt = TokenClient::new(&fixture.e, &fixture.blnt);
    blnt.approve(&fixture.owner, &fixture.contract, &899, &u32::MAX);

    assert!(fixture.client().try_claim_and_fund().is_err());
    let token = MockTokenClient::new(&fixture.e, &fixture.blnt);
    assert_eq!(token.balance(&fixture.source), 900);
    assert_eq!(token.balance(&fixture.owner), 0);
    assert_eq!(token.balance(&fixture.contract), 0);
    assert_eq!(token.balance(&fixture.manager), 0);
    assert_eq!(token.allowance(&fixture.owner, &fixture.contract), 899);
}

#[test]
fn expired_allowance_rolls_back_the_source_claim_and_transfers() {
    let fixture = Fixture::create(900);
    let blnt = TokenClient::new(&fixture.e, &fixture.blnt);
    let expiration_ledger = fixture.e.ledger().sequence() + 1;
    blnt.approve(&fixture.owner, &fixture.contract, &900, &expiration_ledger);
    fixture
        .e
        .ledger()
        .set_sequence_number(expiration_ledger + 1);

    assert!(fixture.client().try_claim_and_fund().is_err());
    let token = MockTokenClient::new(&fixture.e, &fixture.blnt);
    assert_eq!(token.balance(&fixture.source), 900);
    assert_eq!(token.balance(&fixture.owner), 0);
    assert_eq!(token.balance(&fixture.contract), 0);
    assert_eq!(token.balance(&fixture.manager), 0);
    assert_eq!(token.allowance(&fixture.owner, &fixture.contract), 0);
}

#[test]
fn claim_and_fund_does_not_depend_on_operational_manager_scope() {
    let fixture = Fixture::create_as_owner_with_manager_scope(900, 0);
    let manager = MockManagerClient::new(&fixture.e, &fixture.manager);
    assert_eq!(manager.owner(), fixture.owner);
    assert_eq!(manager.manager().id, fixture.operator);
    assert_eq!(manager.manager().scope, 0);

    assert_eq!(fixture.client().claim_and_fund(), 900);
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.manager),
        300
    );
}

#[test]
#[should_panic]
fn constructor_rejects_owner_as_high_scope_manager_without_ownership() {
    Fixture::create_as_nonowner_high_scope_manager(900);
}

#[test]
#[should_panic]
fn constructor_rejects_owner_without_manager_ownership_or_role() {
    Fixture::create_without_manager_access(900);
}

#[test]
fn constructor_accepts_owner_with_separate_low_scope_manager() {
    let fixture = Fixture::create_as_owner_with_manager_scope(900, 0);
    let manager = MockManagerClient::new(&fixture.e, &fixture.manager);
    assert_eq!(manager.owner(), fixture.owner);
    assert_ne!(manager.manager().id, fixture.owner);
    assert_eq!(manager.manager().scope, 0);
}

#[test]
fn premature_owner_transfer_rolls_back_the_entire_claim() {
    let fixture = Fixture::create(900);
    let yieldblox = Address::generate(&fixture.e);
    let manager = MockManagerClient::new(&fixture.e, &fixture.manager);
    manager.set_owner(&yieldblox);
    assert_eq!(manager.manager().id, fixture.operator);

    assert!(fixture.client().try_claim_and_fund().is_err());
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.owner),
        0
    );
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.manager),
        0
    );
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.source),
        900
    );
    assert_eq!(fixture.client().get_totals().custody, 0);
}

#[test]
fn operational_manager_role_does_not_replace_required_ownership() {
    let fixture = Fixture::create(900);
    let yieldblox = Address::generate(&fixture.e);
    let manager = MockManagerClient::new(&fixture.e, &fixture.manager);
    manager.set_owner(&yieldblox);
    manager.set_manager(&fixture.owner, &2);

    assert!(fixture.client().try_claim_and_fund().is_err());
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.source),
        900
    );
}

#[test]
fn manager_route_state_does_not_block_blnt_funding() {
    let fixture = Fixture::create(900);
    let manager = MockManagerClient::new(&fixture.e, &fixture.manager);
    manager.set_pool_allowed(&false);
    manager.set_claim_failure(&BackstopTier::FirstLoss, &true);
    manager.set_claim_failure(&BackstopTier::SecondLoss, &true);

    assert_eq!(fixture.client().claim_and_fund(), 900);
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.manager),
        300
    );
    assert_eq!(fixture.client().get_totals().custody, 600);
}

#[test]
fn one_third_manager_funding_rounds_down() {
    let fixture = Fixture::create(905);

    let result = fixture.client().claim_and_fund();
    assert_eq!(result, 905);
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.contract),
        604
    );
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.manager),
        301
    );
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.owner),
        0
    );
}

#[test]
fn small_grant_does_not_require_an_lp_minimum() {
    let fixture = Fixture::create(12);

    let result = fixture.client().claim_and_fund();
    assert_eq!(result, 12);
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.contract),
        8
    );
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.manager),
        4
    );
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
    assert!(fixture.client().try_claim_and_fund().is_err());
    assert!(fixture.client().try_expire_claims().is_err());
    assert_eq!(fixture.client().get_totals().custody, 0);
}

#[test]
fn expire_claims_is_available_immediately_and_refunds_all_custody() {
    let fixture = Fixture::create(900);
    let funder = Address::generate(&fixture.e);
    let token = MockTokenClient::new(&fixture.e, &fixture.blnt);
    token.mint(&funder, &1_100);
    let client = fixture.client();
    client.fund(&funder, &1_000);
    let beneficiary = fixture.beneficiaries.get_unchecked(0);
    assert_eq!(client.get_claim(&beneficiary).payable, 1_000);
    assert!(!client.claims_closed());

    assert_eq!(client.claim(&beneficiary), 1_000);
    assert_eq!(client.fund(&funder, &100), 100);
    assert_eq!(client.get_claim(&beneficiary).payable, 100);

    let owner_before = token.balance(&fixture.owner);
    let totals_before = client.get_totals();
    assert_eq!(client.expire_claims(), 100);
    assert!(client.claims_closed());
    assert_eq!(token.balance(&fixture.owner), owner_before + 100);
    assert_eq!(token.balance(&fixture.contract), 0);
    assert_eq!(client.get_claim(&beneficiary).payable, 0);
    assert!(client.try_claim(&beneficiary).is_err());
    token.mint(&funder, &1);
    assert!(client.try_fund(&funder, &1).is_err());
    let totals_after = client.get_totals();
    assert_eq!(totals_after.claimed, totals_before.claimed);
    assert_eq!(totals_after.outstanding, totals_before.outstanding);
    assert_eq!(totals_after.custody, 0);
    assert!(client.try_expire_claims().is_err());
}

#[test]
fn expiring_claims_requires_owner_authorization() {
    let fixture = Fixture::create_without_auth(900);
    let token = MockTokenClient::new(&fixture.e, &fixture.blnt);
    token.mint(&fixture.contract, &100);

    assert!(fixture.client().try_expire_claims().is_err());
    assert!(!fixture.client().claims_closed());
    assert_eq!(token.balance(&fixture.contract), 100);
    assert_eq!(token.balance(&fixture.owner), 0);
}

#[test]
fn grant_funding_remains_open_until_explicit_closure() {
    let fixture = Fixture::create(900);
    let token = MockTokenClient::new(&fixture.e, &fixture.blnt);
    let result = fixture.client().claim_and_fund();
    assert_eq!(result, 900);
    assert_eq!(fixture.client().get_totals().custody, 600);
    assert_eq!(fixture.client().expire_claims(), 600);
    assert!(fixture.client().claims_closed());
    assert_eq!(token.balance(&fixture.contract), 0);
}

#[test]
fn zero_custody_expiration_closes_claims_and_blocks_lockup_funding() {
    let fixture = Fixture::create(900);
    let token = MockTokenClient::new(&fixture.e, &fixture.blnt);
    assert_eq!(fixture.client().expire_claims(), 0);
    assert!(fixture.client().claims_closed());
    let result = fixture.client().claim_and_fund();
    assert_eq!(result, 900);
    assert_eq!(token.balance(&fixture.owner), 600);
    assert_eq!(token.balance(&fixture.source), 0);
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
fn constructor_accepts_deployment_supplied_allocations_and_derives_total() {
    let fixture = Fixture::create(900);
    let config = fixture.client().get_config();
    let first = Address::generate(&fixture.e);
    let second = Address::generate(&fixture.e);
    let allocations = vec![
        &fixture.e,
        (first.clone(), 100_i128),
        (second.clone(), 200_i128),
    ];

    let contract = register_with_allocations(&fixture.e, config, allocations);
    let client = BlntLockupBackfillContractClient::new(&fixture.e, &contract);
    assert_eq!(client.get_config().total_allocation, 300);
    assert_eq!(client.get_totals().allocated, 300);
    assert_eq!(client.get_claim(&first).allocation, 100);
    assert_eq!(client.get_claim(&second).allocation, 200);
}

#[test]
#[should_panic]
fn constructor_rejects_duplicate_beneficiaries() {
    let fixture = Fixture::create(900);
    let beneficiary = Address::generate(&fixture.e);
    register_with_allocations(
        &fixture.e,
        fixture.client().get_config(),
        vec![
            &fixture.e,
            (beneficiary.clone(), 100_i128),
            (beneficiary, 200_i128),
        ],
    );
}

#[test]
#[should_panic]
fn constructor_rejects_nonpositive_allocations() {
    let fixture = Fixture::create(900);
    register_with_allocations(
        &fixture.e,
        fixture.client().get_config(),
        vec![&fixture.e, (Address::generate(&fixture.e), 0_i128)],
    );
}

#[test]
#[should_panic]
fn constructor_rejects_contract_self_beneficiary() {
    let fixture = Fixture::create(900);
    let config = fixture.client().get_config();
    let contract = Address::generate(&fixture.e);
    fixture.e.register_at(
        &contract,
        BlntLockupBackfillContract,
        (
            config.blnt,
            config.source_backfill,
            config.owner,
            config.manager,
            vec![&fixture.e, (contract.clone(), 1_i128)],
        ),
    );
}

#[test]
#[should_panic]
fn constructor_rejects_allocation_sum_overflow() {
    let fixture = Fixture::create(900);
    register_with_allocations(
        &fixture.e,
        fixture.client().get_config(),
        vec![
            &fixture.e,
            (Address::generate(&fixture.e), i128::MAX),
            (Address::generate(&fixture.e), 1_i128),
        ],
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
fn claim_and_fund_caps_lockup_funding_at_the_remaining_gap() {
    let fixture = Fixture::create(900);
    let funder = Address::generate(&fixture.e);
    let initial_funding = TOTAL_ALLOCATION - 100;
    MockTokenClient::new(&fixture.e, &fixture.blnt).mint(&funder, &initial_funding);
    fixture.client().fund(&funder, &initial_funding);
    let result = fixture.client().claim_and_fund();
    assert_eq!(result, 900);
    assert_eq!(
        MockTokenClient::new(&fixture.e, &fixture.blnt).balance(&fixture.owner),
        500
    );
    assert_eq!(fixture.client().get_totals().custody, TOTAL_ALLOCATION);
}
