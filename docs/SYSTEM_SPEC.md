# BLNT Lockup Backfill Contract System Specification

## 1. Purpose

The BLNT lockup backfill contract distributes voluntarily contributed BLNT to
the economic owners of four immutable `blend-lockup v1.0.0` contracts whose
pre-incident Comet LP positions could not be claimed correctly from the
deployed BLNT backfill contract.

Three lockup contracts were assigned BLNT directly, but their code exposes no
call path capable of authorizing the deployed backfill contract's
`claim_backfill(lockup)` invocation. One lockup contract was omitted entirely
because its complete LP position was held through the V1 backstop, whose
custody was not flattened to its depositors in the production claimant
manifest. Two included lockups also had small or material V1 portions omitted.

This contract is a separately funded replacement distribution. It MUST NOT
modify the deployed backfill contract, consume its accounting, mint BLNT, or
claim that it changes the original 74-million-BLNT allocation. BLNT deposited
here is a voluntary contribution from BLNT already in circulation.

Normative terms `MUST`, `MUST NOT`, `SHOULD`, and `MAY` are acceptance
criteria.

## 2. Immutable mainnet bindings

The production deployment MUST bind:

- BLNT Stellar Asset Contract:
  `CCBNWP5L7ZSXA3DBWCMFAVKFVRW3ZHWO7B5J557W42O64DDZNPJP7VDS`;
- canonical deployed BLNT backfill contract:
  `CANIE64BDFKLHROLOIY2KTQVBWUZNUKZP75DLXC272QM7TIN5N3M4LVA`;
- immutable owner account supplied as a constructor argument and
  recorded only in the deployment record, not hardcoded in repository source;
  the owner is also the source contributor-grant recipient and refund recipient;
- V3 backstop:
  `CAHCUKBPTW5DR5RNXOHCTJD5U76A6EEZXYZAQFRQEEQRXQ3CRUZ6Z3MO`;
- YieldBlent pool:
  `CAUTPSQDAJEUV5XEU3EXFVYRGDJLEJGVTFJCFZCSPTDCREAMZ573VF4D`;
- YieldBlent FirstLoss token, the canonical BLNT:XLM Comet V1.1 LP:
  `CA5HX3D47PQZWC2E7VWXCKH4534P2NOJEOIXAWU47MI4QV2FQUYY3FLC`;
- YieldBlent SecondLoss token, the canonical BLNT:USDC Comet V1.1 LP:
  `CDQMIKIHKA5UY3FQJ2UDTLNX3GOPD3GPNZWSPTOBVOHEUMQJQXOPC7Y3`;
- a verified backstop-manager instance deployed and configured before this
  contract, which the immutable owner controls throughout the backfill period;
  its production contract address remains
  an open deployment binding until that instance is deployed and independently
  verified;
- the source backfill contract's immutable contributor-grant vesting start
  returned by `get_grant_vesting_start()`; and
- the source backfill contract's immutable contributor-grant vesting end
  returned by `get_grant_vesting_end()`.

Construction MUST verify that the source contract reports the bound BLNT
token, that its contributor-grant vesting end is exactly 360 days after its
contributor-grant vesting start, and that the start is not later than the new
contract's construction timestamp. The replacement vesting epoch MUST use
those source contributor-grant timestamps; it MUST NOT restart when this
contract is deployed or funded.

Before production construction, deployment verification MUST establish that
the immutable owner owns the bound manager, that the manager recognizes the
V3 backstop and YieldBlent pool, that the owner can invoke high-scope Comet
joins and medium-scope backstop deposits, and that the two tier tokens resolve
to the exact canonical LP addresses above. A mismatch MUST block deployment.
The owner MUST retain backstop-manager ownership while contributor-grant
claims still require the atomic route. A separate operational manager,
including YBX DAO or YSC, MAY manage the position under its configured scope
without gaining authority over owner claims. Backstop-manager ownership and
management MAY be transferred after the backfill period is complete.

Network-specific test deployments MAY use different bindings, but tests and
deployment records MUST label them unambiguously and preserve all production
invariants.

## 3. Evidence and allocations

All amounts use seven-decimal raw units. The share-based allocation rate is
derived from the same production backfill basis:

`74,000,000 BLNT / 14,610,061.9813817 LP-equivalent shares`.

The production contract MUST contain exactly these immutable replacement
allocations:

1. Lockup
   `CBJJOIRNZOFH5NGTU2SLFEAK4TB5P2KSOCOWTSWCTYNBSMMIBYFMFETV`
   - owner and beneficiary:
     `GBDVBOWFPYWJS3BVS4H4YYOGN7UTMR4TKKPJ77YK2N7LD7OVYI35NN52`;
   - pre-incident LP-equivalent shares: `175,558.4040441`;
   - replacement allocation: `889,203.7498416 BLNT`.
2. Lockup
   `CCJGQ7FUZ7KTRRBMW3KJ2IUNKN2TTJ6HKEKUZVORQBALMJNTUT6XVYVH`
   - owner and beneficiary:
     `GCQEFBTWQMZMKS2PAFUKQN7QE5YQ4SFSU5PTRAV7WETVT6LNKDE5ARA7`;
   - pre-incident LP-equivalent shares: `149,999.1252928`;
   - replacement allocation: `759,745.9398743 BLNT`.
3. Lockup
   `CD7UCK6QPQVXGHI36HEVNAJXRO6GPFWSVBJDXK37ENL3DZ2NKEX5WM4S`
   - owner recorded at the cutoff:
     `GBANEKU4QTP7KP3AX5ZZKZKNAHTXF37B62DDJUFNYJAFZDSOV5GDTR6N`;
   - that owner was merged on 2026-10-11 into the immutable beneficiary:
     `GANNTI6OTKR6WSOVOIGUMT7GMSSEPV3UL3W4VI22VLFW3NT5ABOZ2YFB`;
   - pre-incident LP-equivalent shares: `150,068.0000001`;
   - replacement allocation: `760,094.7904368 BLNT`.
4. Lockup
   `CDJDJJYJU5DDX267RYB3RWYS6FFJXAXQABV2YNXJ3RGFEIR3X7B43UPQ`
   - owner and beneficiary:
     `GBJXBKZ643REPENN3NBTVGNH3DCDJGVYQE3QDA37WMAERN4JE4HTZQCW`;
   - pre-incident LP-equivalent shares: `263,612.1321711`;
   - replacement allocation: `1,335,196.1001616 BLNT`.

The aggregate replacement allocation MUST be exactly
`3,744,240.5803143 BLNT`. Construction MUST reject any missing, duplicate,
non-positive, or additional allocation and any aggregate mismatch.
The contract implementation MUST compare every constructor-supplied
beneficiary and amount against this canonical table; matching only the entry
count and aggregate is insufficient.

These are full share-based replacement amounts. They intentionally include
the V1 portions omitted from the production claimant manifest. The source
backfill allocations assigned to three lockup contract addresses remain
unchanged and inaccessible through this contract.

## 4. Vesting and claim closure

For beneficiary allocation `A`, source vesting start `S`, source vesting end
`E`, and current ledger timestamp `T`, cumulative vested BLNT MUST equal:

- zero when `T <= S`;
- `floor(A * (T - S) / (E - S))` when `S < T < E`; and
- exactly `A` when `T >= E`.

Vesting MUST accrue per second from the original source contract's deployment
epoch. Funding time, claim time, and this contract's deployment time MUST NOT
alter vesting.

Construction MUST leave claims open. Time alone MUST NOT expire or otherwise
change claims, funding, or vesting. Claims remain open until the designated
owner explicitly closes them through the refund entrypoint defined below.

## 5. Funding

### 5.1 Direct funding

`fund(from, amount) -> i128` MUST:

1. require `from` authorization;
2. require a positive amount;
3. reject a transfer that would make contract BLNT custody exceed aggregate
   allocation less aggregate successful claims;
4. transfer exactly `amount` BLNT from `from` to the contract;
5. verify the exact balance increase; and
6. emit the funding source, amount, and post-funding balance.

Any address MAY fund the contract. Funding creates no ownership, withdrawal,
refund, governance, or allocation rights.

Direct funding MUST fail after claims have been explicitly closed so newly
transferred BLNT cannot become stranded.

### 5.2 Atomic owner claim, lockup funding, and YieldBlent deposit

`claim_grant_and_fund() -> GrantFundingResult` MUST:

1. require authorization by the immutable owner;
2. invoke `claim_grant(owner)` on the canonical source backfill
   contract and read the exact positive amount `C` returned;
3. calculate the manager BLNT route as `M = floor(C / 3)`;
4. calculate the FirstLoss BLNT budget as `F = floor(M / 2)` and the SecondLoss
   BLNT budget as `S = M - F`, assigning any odd raw unit to SecondLoss;
5. calculate the remaining replacement-claim funding gap as aggregate
   outstanding entitlement less current contract BLNT custody, floored at
   zero;
6. while claims are open, set the lockup-funding amount `L` to the lesser of
   that funding gap and `C - M`; after claims are closed, set `L = 0`;
7. transfer exactly `L` BLNT from the owner to this contract and verify exact
   receipt;
8. when the owner still owns the bound manager, derive the greatest
   positive proportional LP output from each live Comet reserve composition
   whose rounded BLNT input does not exceed `F` or `S`;
9. derive each proportional counterasset input using Comet V1.1's upward
   rounding and set its immutable maximum to `ceil(derived_input * 1.01)`;
10. transfer only the exact positive BLNT amounts `F_used <= F` and
   `S_used <= S` derived for those joins and verify exact receipt;
11. use exactly `F_used` BLNT and sufficient XLM held by the manager to join the
   canonical BLNT:XLM Comet V1.1 LP;
12. deposit every LP share minted by that join into YieldBlent FirstLoss for
    the manager contract;
13. use exactly `S_used` BLNT and sufficient USDC held by the manager to join
    the canonical BLNT:USDC Comet V1.1 LP;
14. deposit every LP share minted by that join into YieldBlent SecondLoss for
    the manager contract;
15. leave `M - F_used - S_used` in the owner wallet without transferring it to
    the manager; and
16. return the grant amount claimed, lockup funding, whether both backstop
    deposits completed, and total BLNT retained by the owner.

If the owner no longer owns the manager, or if the bound backstop or pool has
been removed from the manager allowlists, the entrypoint MUST skip both joins
and deposits. It MUST still complete the source grant claim and automatically
bounded lockup funding,
and the entire `M` allocation MUST remain in the owner wallet. A manager
fallback is not permitted for a substituted backstop tier token or Comet pair;
those immutable binding mismatches MUST fail the call.

The owner MUST retain `C - L - F_used - S_used`. This includes both the
ordinary post-manager remainder and any unconsumed part of `M`; on the
fallback path it includes all of `M`. The one-third route is measured in BLNT;
the XLM and USDC required to mint the LPs are separate manager-provided
capital. When routing is available, the manager MUST be sufficiently
pre-funded with both counterassets before invocation.

When manager access is available, the source claim, lockup contribution, BLNT
transfer to the manager, both LP joins, and both backstop deposits MUST be one
atomic call tree. Failure of any authorization, balance check, slippage bound,
LP join, tier resolution, or backstop deposit MUST roll back the entire source
claim and every transfer. Before claiming the contributor grant, every route
invocation MUST determine whether the owner still owns the mutable external
manager and whether the manager still recognizes the bound backstop and pool.
It MUST also revalidate that the backstop still resolves the expected tier
tokens and each Comet token pair
remains exact. A later loss of owner access or allowlist removal therefore
selects the owner-retention fallback instead of redirecting or stranding
assets. Direct funding and beneficiary claims remain independent of the
manager.

For each tier, the implementation MUST query the bound Comet pool's current
total LP supply, BLNT reserve, counterasset reserve, and token list. It MUST
reproduce Comet V1.1 proportional-join upward rounding, derive the greatest LP
output whose rounded BLNT input remains within the tier budget, use that exact
BLNT amount as the BLNT maximum, and use 101% of the derived counterasset input
rounded upward as the XLM or USDC maximum. A zero output, zero reserve, zero
supply, invalid pair, arithmetic overflow, or input outside those limits MUST
fail atomically.

The implementation MUST snapshot the manager's BLNT, counterasset, and LP
balances around each join, verify that FirstLoss consumes exactly `F_used`,
verify that SecondLoss consumes exactly `S_used`, enforce the fixed 1%
counterasset ceiling, verify each minted LP delta, and deposit exactly that
delta. `F_used` and `S_used` MUST NOT exceed their respective budgets. The
difference `M - F_used - S_used` MUST never leave the owner. The manager's
liquid balances of the two LP tokens and BLNT MUST return to their respective
pre-route values after the deposits. No routed BLNT or newly minted LP token
may remain idle in the manager.

The entrypoint MUST NOT accept caller-selected route amounts or slippage,
substitute legacy BLND, select a different pool or tier, or route funds to an
unverified manager.

The successful call MUST emit detailed accounting containing the grant amount,
lockup funding, backstop budget and unspent portion, whether the backstop route
completed, owner retention, and the BLNT, XLM or USDC, LP shares, and backstop
shares used for each tier. These audit fields MUST NOT expand the compact
`GrantFundingResult` return value.

BLNT already claimed before this contract exists MUST be contributed through
`fund`; it cannot be retroactively coupled to an earlier source claim.
The intended funding sequence is an initial direct contribution of already
claimed BLNT followed by automatically bounded top-ups from later
contributor-grant claims. Once the replacement obligation is fully funded,
the inferred `L` MUST remain zero so the owner can continue claiming the
contributor grant and completing the manager route without overfunding this
contract.

After claims have been explicitly closed, the atomic owner route MUST infer a
zero lockup-funding amount. It MAY continue to claim the owner's source grant
and execute or skip the independently configured manager route under the
ordinary rules.

## 6. Beneficiary claims under partial funding

`claim(beneficiary) -> i128` MUST:

1. require `beneficiary` authorization;
2. reject an address outside the immutable beneficiary table;
3. calculate vested-and-unclaimed entitlement as cumulative vested allocation
   less that beneficiary's successful prior claims;
4. calculate available BLNT as the contract's current BLNT balance;
5. transfer `min(vested_and_unclaimed, available_blnt)` to the same
   beneficiary;
6. reject when that minimum is zero;
7. increment beneficiary and aggregate progress by exactly the transferred
   amount; and
8. verify the exact contract debit before completing.

If custody is insufficient, a successful claim MUST leave the unpaid remainder
in the beneficiary's entitlement. The beneficiary MAY claim again after more
BLNT is funded. A partial payout MUST NOT mark an allocation complete.

Available custody is shared rather than reserved per beneficiary. Among
simultaneously vested claims, successful transactions consume available funds
in ledger order. This may delay later claimants but MUST NOT reduce their
recorded entitlement. Funding MUST eventually reach the aggregate allocation
for every beneficiary to receive the full amount.

## 7. Claim closure and refund

`expire_claims() -> i128` MUST atomically:

1. require authorization by the immutable owner;
2. reject if claims have already been closed;
3. mark claims permanently closed;
4. transfer the contract's complete remaining BLNT balance, if positive, to
   that owner;
5. verify the exact contract debit and owner balance increase for a positive
   refund;
6. leave immutable allocations and historical claimed progress unchanged; and
7. emit the owner, refunded amount, and aggregate claimed amount.

The entrypoint MUST NOT accept a caller-selected recipient, transfer another
asset, or confer refund rights on a funder. Before the entrypoint succeeds,
beneficiary claims and funding remain available. After it succeeds,
beneficiary claims and new lockup funding MUST fail permanently even when the
refund amount was zero. A later direct token transfer is outside the supported
API and cannot be recovered through a second closure call.

## 8. Accounting and views

The contract SHOULD expose only the views required to independently verify:

- BLNT and source-backfill bindings;
- immutable owner;
- backstop-manager, V3 backstop, YieldBlent pool, and both canonical Comet LP
  bindings;
- source vesting start and end;
- whether claims have been closed;
- each beneficiary's immutable allocation, cumulative vested amount,
  cumulative claimed amount, and currently payable amount;
- aggregate allocated, claimed, outstanding, funded-custody, and funding-gap
  amounts; and
- the complete bounded beneficiary list.

`currently payable` MUST reflect both vesting and current contract custody.
`outstanding` MUST include vested and unvested unclaimed allocations.
`funding gap` MUST equal outstanding allocation less current BLNT custody.
After closure, `outstanding` and `funding gap` remain historical accounting
values and do not reopen claims or authorize new funding.

## 9. Security and lifecycle

The contract MUST:

- use checked `i128` arithmetic and reject overflow or inconsistent state;
- use a reentrancy lock around every external token or source-contract call;
- update claim progress before transfer and rely on atomic rollback on failure;
- verify exact token balance deltas;
- reject its own address as a funder or beneficiary;
- maintain and renew instance and persistent-state TTLs through normal calls;
- contain no administrator, upgrade, allocation mutation, arbitrary
  withdrawal, mint, burn, or arbitrary-call entrypoint; the sole
  recovery path is the owner-authorized, BLNT-only claim closure and refund
  defined above; and
- include source repository metadata in release WASM.

The supported `fund` paths MUST prevent overfunding, and deployment
instructions MUST warn against direct token transfers. Any BLNT sent directly
to the contract remains unavailable until beneficiary claims consume it or the
one-time closure returns the remaining balance to the owner.

## 10. Required tests

Tests MUST cover:

- exact production bindings and allocation aggregate;
- source vesting start, midpoint flooring, vesting end, and post-end claims;
- deployment after `S` without restarting vesting;
- each beneficiary and the merged-owner successor;
- unauthorized and unknown-beneficiary claims;
- zero custody, partial custody, repeated partial claims, and later refill;
- first-come custody consumption without entitlement loss;
- direct funding, overfunding rejection, and exact balance verification;
- atomic source claim plus funding, including rollback of each attempted-route
  failure mode;
- exact one-third manager budgeting and odd-unit assignment to the SecondLoss
  budget;
- equal BLNT budgets across the BLNT:XLM and BLNT:USDC joins before an odd raw
  unit is assigned to SecondLoss;
- live-reserve proportional-join derivation matching Comet V1.1 upward
  rounding, including a budget that cannot mint one LP raw unit;
- exact BLNT maxima, fixed 1% XLM/USDC maxima rounded upward, and all unused
  BLNT retained by the owner without entering manager custody;
- successful construction and routing while the owner owns the manager and a
  separate operational manager is configured;
- rejection when the owner is the configured manager, at any scope, but does
  not own the manager contract;
- source claim and lockup funding with the full one-third retained by the owner
  after owner access or required allowlist access is removed prematurely;
- manager counterasset underfunding and per-join slippage failures;
- rejection of substituted managers, pools, tiers, and LP tokens;
- exact LP balance deltas and complete FirstLoss and SecondLoss deposits;
- rollback when either the first or second join or deposit fails;
- automatic lockup funding capped by both the post-manager remainder and the
  current aggregate funding gap;
- already-claimed owner funds through `fund`;
- transfer failures, reentrancy, overflow, and state rollback;
- full funding and full distribution with zero remaining entitlement;
- claims and funding remaining open until explicit closure;
- rejection of closure without owner authorization;
- exact full-custody refund to the owner at closure, including balance-delta
  verification, zero-custody closure, repeated-closure rejection, and
  unchanged claim progress;
- permanent rejection of beneficiary claims and new lockup funding after
  closure; and
- TTL extension and restoration behavior.

Production deployment acceptance MUST independently reproduce the allocation
inputs, optimized WASM hash, constructor arguments, source vesting timestamps,
and contract bindings before any BLNT is funded.
