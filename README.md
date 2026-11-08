# BLNT Lockup Backfill Contract

An immutable Soroban contract for voluntarily funded replacement BLNT
backfill claims owed to the economic owners of four `blend-lockup v1.0.0`
contracts.

The constructor accepts immutable beneficiary/allocation pairs, rejects
duplicates and non-positive amounts, and derives the aggregate allocation used
by funding and claim accounting. Production deployment tooling and review are
responsible for verifying the exact four canonical pairs and aggregate before
instantiation.

For production, validate the committed manifest and generate the complete
constructor argument from the same checked input:

```sh
make validate-production-allocations
make production-allocation-args
```

Production deployment tooling must consume the second command's output as the
`allocation_list` argument rather than constructing that argument separately.

The contract preserves the deployed BLNT backfill contract's original
360-day contributor-grant vesting epoch, supports partial funding without
erasing unpaid entitlement, and keeps replacement claims open until the
immutable owner explicitly closes them. The owner may close all remaining
claims at any time and atomically return the complete remaining BLNT custody to
the owner wallet. The no-argument `claim_and_fund()` entrypoint provides
an atomic owner route that:

1. claims the immutable owner's currently vested contributor grant;
2. automatically funds as much of the outstanding lockup-owner obligation as
   possible from the two-thirds not reserved for the manager route;
3. budgets one-third of the source claim equally by BLNT amount across the
   canonical BLNT:XLM and BLNT:USDC Comet V1.1 pools;
4. derives both proportional joins from current Comet reserves and supply with
   a fixed 1% XLM/USDC slippage ceiling;
5. deposits the resulting LP shares into YieldBlent FirstLoss and SecondLoss;
6. claims accrued YieldBlent emissions for both tiers; and
7. transfers only the exact BLNT consumed by each join, leaving proportional
   join rounding and all remaining BLNT in the owner wallet.

Construction verifies the initial backstop-manager ownership and allowlists as
well as the immutable tier-token and Comet-pair bindings. Each later
contributor-grant claim attempts the complete route and relies on the manager's
live authorization and allowlist enforcement. A separate manager such as YBX
DAO or YSC may operate the position within its configured scope but cannot
authorize contributor-grant claims. If any manager authorization, allowlist,
join, or deposit operation fails, the source claim and every related transfer
roll back atomically so the configuration can be corrected and retried.

The intended funding sequence is a direct initial contribution from BLNT
already held by the owner, followed by optional top-ups from later vested
contributor-grant claims. A zero top-up remains valid after the replacement
claims are fully funded.

The contract cannot mint BLNT and has no administrator, upgrade, allocation
mutation, arbitrary recovery, or caller-selected refund recipient. See [the
system specification](docs/SYSTEM_SPEC.md) for the normative requirements and
deployment bindings.

## Development

The workspace pins Rust 1.91.1 and Soroban SDK 27.0.3.

```sh
make test
make build
```

No production deployment is authorized or recorded yet. The backstop-manager
binding remains open until its deployed instance and integration are
independently verified.
