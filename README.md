# BLNT Lockup Backfill Contract

An immutable Soroban contract for voluntarily funded replacement BLNT
backfill claims owed to the economic owners of four `blend-lockup v1.0.0`
contracts.

The contract hard-validates the four canonical beneficiary/allocation pairs
at construction; a substituted address or amount cannot be deployed with the
same aggregate total.

The contract preserves the deployed BLNT backfill contract's original
360-day contributor-grant vesting epoch, supports partial funding without
erasing unpaid entitlement, and keeps replacement claims open until the
immutable owner explicitly closes them. The owner may close all remaining
claims at any time and atomically return the complete remaining BLNT custody to
the owner wallet. The no-argument `claim_grant_and_fund()` entrypoint provides
an atomic owner route that:

1. claims the immutable owner's currently vested contributor grant;
2. automatically funds as much of the outstanding lockup-owner obligation as
   possible from the two-thirds not reserved for the manager route;
3. budgets one-third of the source claim equally by BLNT amount across the
   canonical BLNT:XLM and BLNT:USDC Comet V1.1 pools;
4. derives both proportional joins from current Comet reserves and supply with
   a fixed 1% XLM/USDC slippage ceiling;
5. deposits the resulting LP shares into YieldBlent FirstLoss and SecondLoss;
   and
6. transfers only the exact BLNT consumed by each join, leaving proportional
   join rounding and all remaining BLNT in the owner wallet.

During backfill, the immutable owner must own the backstop manager. A
separate manager such as YBX DAO or YSC may operate the position within its
configured scope while that ownership remains in place, but cannot authorize
contributor-grant claims. If backstop-manager ownership or the required
backstop/pool allowlist is removed prematurely, contributor-grant claims
continue without the YieldBlent route and its complete one-third budget
remains in the owner wallet. Backstop-manager ownership and management may be
transferred after backfill is complete.

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
