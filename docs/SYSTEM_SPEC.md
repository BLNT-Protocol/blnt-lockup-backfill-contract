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
- designated donor account supplied as an immutable constructor argument and
  recorded only in the deployment record, not hardcoded in repository source;
- V3 backstop:
  `CAHCUKBPTW5DR5RNXOHCTJD5U76A6EEZXYZAQFRQEEQRXQ3CRUZ6Z3MO`;
- YieldBlent pool:
  `CAUTPSQDAJEUV5XEU3EXFVYRGDJLEJGVTFJCFZCSPTDCREAMZ573VF4D`;
- YieldBlent FirstLoss token, the canonical BLNT:XLM Comet V1.1 LP:
  `CA5HX3D47PQZWC2E7VWXCKH4534P2NOJEOIXAWU47MI4QV2FQUYY3FLC`;
- YieldBlent SecondLoss token, the canonical BLNT:USDC Comet V1.1 LP:
  `CDQMIKIHKA5UY3FQJ2UDTLNX3GOPD3GPNZWSPTOBVOHEUMQJQXOPC7Y3`;
- a verified backstop-manager instance deployed and configured before this
  contract, owned during backfill by the designated donor and operated by a
  high-scope YieldBlox manager; its production contract and YieldBlox manager
  addresses remain open deployment bindings until that instance is deployed
  and independently verified;
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
the designated donor owns the bound manager, that YieldBlox is its configured
high-scope manager, that the manager recognizes the V3 backstop and YieldBlent
pool, can invoke high-scope Comet joins and medium-scope backstop deposits, and
resolves the two tier tokens to the exact canonical LP addresses above. A
mismatch MUST block deployment. The designated donor MAY transfer ownership to
YieldBlox after the backfill period is complete; ownership MUST NOT be
transferred while contributor-grant claims still require the atomic route.

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

## 4. Vesting

For beneficiary allocation `A`, source vesting start `S`, source vesting end
`E`, and current ledger timestamp `T`, cumulative vested BLNT MUST equal:

- zero when `T <= S`;
- `floor(A * (T - S) / (E - S))` when `S < T < E`; and
- exactly `A` when `T >= E`.

Vesting MUST accrue per second from the original source contract's deployment
epoch. Funding time, claim time, and this contract's deployment time MUST NOT
alter vesting. Claims MUST remain available indefinitely after vesting ends.

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

### 5.2 Atomic donor claim, lockup funding, and YieldBlent deposit

`claim_source_route_and_fund(...) -> RouteResult` MUST:

1. require authorization by the immutable designated donor;
2. invoke `claim_grant(designated_donor)` on the canonical source backfill
   contract and read the exact positive amount `C` returned;
3. calculate the manager BLNT route as `M = floor(C / 3)`;
4. calculate the FirstLoss BLNT budget as `F = floor(M / 2)` and the SecondLoss
   BLNT budget as `S = M - F`, assigning any odd raw unit to SecondLoss;
5. accept an explicit non-negative lockup-funding amount `L` and require
   `L <= C - M`;
6. reject `L` if it would overfund outstanding replacement entitlements;
7. transfer exactly `L` BLNT from the donor to this contract and verify exact
   receipt;
8. when the donor can still operate the bound manager, transfer only the exact
   positive BLNT amounts `F_used <= F` and `S_used <= S` required by the two
   caller-bounded proportional joins and verify exact receipt;
9. use exactly `F_used` BLNT and sufficient XLM held by the manager to join the
   canonical BLNT:XLM Comet V1.1 LP;
10. deposit every LP share minted by that join into YieldBlent FirstLoss for
    the manager contract;
11. use exactly `S_used` BLNT and sufficient USDC held by the manager to join
    the canonical BLNT:USDC Comet V1.1 LP;
12. deposit every LP share minted by that join into YieldBlent SecondLoss for
    the manager contract; and
13. leave `M - F_used - S_used` in the donor wallet without transferring it to
    the manager; and
14. return whether the manager route ran, the source amount, manager budget and
    amount retained, lockup funding, per-tier BLNT consumed, LP shares minted,
    and backstop shares received.

If the donor no longer owns the manager and is no longer its configured
high-scope manager, or if the bound backstop or pool has been removed from the
manager allowlists, the entrypoint MUST skip both joins and deposits. It MUST
still complete the source grant claim and requested lockup funding, and the
entire `M` allocation MUST remain in the donor wallet. Caller-supplied join
parameters are ignored on this fallback path. A manager fallback is not
permitted for a substituted backstop tier token or Comet pair; those immutable
binding mismatches MUST fail the call.

The donor MUST retain `C - L - F_used - S_used`. This includes both the
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
invocation MUST determine whether the donor still owns the mutable external
manager or remains its configured high-scope manager and whether the manager
still recognizes the bound backstop and pool. It MUST also revalidate that the
backstop still resolves the expected tier tokens and each Comet token pair
remains exact. A later loss of donor access or allowlist removal therefore
selects the donor-retention fallback instead of redirecting or stranding
assets. Direct funding and beneficiary claims remain independent of the
manager.

The caller MUST provide the exact positive BLNT input, bounded LP output, and
bounded counterasset input for both joins. Because a proportional Comet join
mints an integer number of LP raw units, the caller SHOULD obtain the exact
BLNT inputs through a recent simulation. The implementation MUST snapshot the
manager's BLNT and both LP balances around each join, verify that FirstLoss
consumes exactly `F_used`, verify that SecondLoss consumes exactly `S_used`,
verify each minted LP delta, and deposit exactly that delta. `F_used` and
`S_used` MUST NOT exceed their respective budgets. The difference
`M - F_used - S_used` MUST never leave the donor. The manager's liquid balances
of the two LP tokens and BLNT MUST return to their respective pre-route values
after the deposits. No routed BLNT or newly minted LP token may remain idle in
the manager.

The entrypoint MUST NOT infer the lockup contribution, transfer more than the
explicit `L`, substitute legacy BLND, select a different pool or tier, or route
funds to an unverified manager.

BLNT already claimed before this contract exists MUST be contributed through
`fund`; it cannot be retroactively coupled to an earlier source claim.
The intended funding sequence is an initial direct contribution of already
claimed BLNT followed by explicit top-ups from later contributor-grant claims.
Once the replacement obligation is fully funded, `L = 0` MUST remain valid so
the donor can continue claiming the contributor grant and completing the
manager route without overfunding this contract.

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

## 7. Accounting and views

The contract SHOULD expose only the views required to independently verify:

- BLNT and source-backfill bindings;
- designated donor;
- backstop-manager, V3 backstop, YieldBlent pool, and both canonical Comet LP
  bindings;
- source vesting start and end;
- each beneficiary's immutable allocation, cumulative vested amount,
  cumulative claimed amount, and currently payable amount;
- aggregate allocated, claimed, outstanding, funded-custody, and funding-gap
  amounts; and
- the complete bounded beneficiary list.

`currently payable` MUST reflect both vesting and current contract custody.
`outstanding` MUST include vested and unvested unclaimed allocations.
`funding gap` MUST equal outstanding allocation less current BLNT custody.

## 8. Security and lifecycle

The contract MUST:

- use checked `i128` arithmetic and reject overflow or inconsistent state;
- use a reentrancy lock around every external token or source-contract call;
- update claim progress before transfer and rely on atomic rollback on failure;
- verify exact token balance deltas;
- reject its own address as a funder or beneficiary;
- maintain and renew instance and persistent-state TTLs through normal calls;
- contain no administrator, upgrade, allocation mutation, sweep, withdrawal,
  recovery, mint, burn, or arbitrary-call entrypoint; and
- include source repository metadata in release WASM.

An accidental excess token transfer made directly to the contract cannot be
recovered. The supported `fund` paths MUST prevent overfunding, and deployment
instructions MUST warn against direct token transfers.

## 9. Required tests

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
- bounded proportional-join rounding with all unused BLNT retained by the
  donor without entering manager custody;
- successful routing while the donor owns the manager contract and YieldBlox
  is a separate high-scope operational manager;
- source claim and lockup funding with the full one-third retained by the donor
  after donor ownership or required allowlist access is removed prematurely;
- manager counterasset underfunding and per-join slippage failures;
- rejection of substituted managers, pools, tiers, and LP tokens;
- exact LP balance deltas and complete FirstLoss and SecondLoss deposits;
- rollback when either the first or second join or deposit fails;
- attempts to fund more than the post-manager remainder of the same source
  claim;
- already-claimed donor funds through `fund`;
- transfer failures, reentrancy, overflow, and state rollback;
- full funding and full distribution with zero remaining entitlement; and
- TTL extension and restoration behavior.

Production deployment acceptance MUST independently reproduce the allocation
inputs, optimized WASM hash, constructor arguments, source vesting timestamps,
and contract bindings before any BLNT is funded.
