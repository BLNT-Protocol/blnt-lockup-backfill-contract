# BLNT Lockup Backfill Contract

An immutable Soroban contract for voluntarily funded replacement BLNT
backfill claims owed to the economic owners of four `blend-lockup v1.0.0`
contracts.

The contract hard-validates the four canonical beneficiary/allocation pairs
at construction; a substituted address or amount cannot be deployed with the
same aggregate total.

The contract preserves the deployed BLNT backfill contract's original
360-day contributor-grant vesting epoch, supports partial funding without
erasing unpaid entitlement, and provides an atomic donor route that:

1. claims the designated donor's currently vested contributor grant;
2. reserves an explicit portion for lockup-owner claims;
3. budgets one-third of the source claim equally by BLNT amount across the
   canonical BLNT:XLM and BLNT:USDC Comet V1.1 pools;
4. deposits the resulting LP shares into YieldBlent FirstLoss and SecondLoss;
   and
5. transfers only the exact BLNT consumed by each join, leaving proportional
   join rounding and all remaining BLNT in the donor wallet.

During backfill, the designated donor owns the backstop manager and YieldBlox
is its high-scope operational manager. The donor can execute the atomic funding
route as owner, while YieldBlox can claim emissions and manage the resulting
backstop positions. If donor access or the required backstop/pool allowlist is
removed prematurely, contributor-grant claims continue without the YieldBlent
route and its complete one-third budget remains in the donor wallet. Ownership
may be transferred to YieldBlox after backfill is complete.

The intended funding sequence is a direct initial contribution from BLNT
already held by the donor, followed by optional top-ups from later vested
contributor-grant claims. A zero top-up remains valid after the replacement
claims are fully funded.

The contract cannot mint BLNT and has no administrator, upgrade, allocation
mutation, sweep, or recovery path. See [the system specification](docs/SYSTEM_SPEC.md)
for the normative requirements and deployment bindings.

## Development

The workspace pins Rust 1.91.1 and Soroban SDK 27.0.3.

```sh
make test
make build
```

No production deployment is authorized or recorded yet. The backstop-manager
binding remains open until its deployed instance and integration are
independently verified.
