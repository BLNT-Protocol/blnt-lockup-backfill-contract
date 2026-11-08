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
the owner wallet. The no-argument `claim_and_fund()` entrypoint atomically:

1. claims the immutable owner's currently vested contributor grant;
2. automatically funds as much of the outstanding lockup-owner obligation as
   possible from the two-thirds not reserved for manager funding;
3. transfers exactly one-third of the source claim to the bound backstop
   manager; and
4. leaves all remaining BLNT in the owner wallet.

Construction and every contributor-grant claim verify that the immutable owner
owns the bound backstop manager. A separate manager such as YBX DAO or YSC may
later join Comet pools, deposit LP shares, and claim emissions within its
configured scope, but cannot authorize contributor-grant claims. Those later
operations are deliberately decoupled so temporary counterasset, reserve,
slippage, or backstop conditions cannot block the source grant claim.

The intended funding sequence is a direct initial contribution from BLNT
already held by the owner, followed by optional top-ups from later vested
contributor-grant claims. A zero top-up remains valid after the replacement
claims are fully funded.

The contract cannot mint BLNT and has no administrator, upgrade, allocation
mutation, arbitrary recovery, or caller-selected refund recipient. See [the
system specification](docs/SYSTEM_SPEC.md) for the normative requirements and
deployment bindings.

The production deployment binds the source-attested YieldBlent backstop
manager
`CAJUD2IQOVKPDZFJZEX2MSG7PKIE2NF4CJQ2ERBU7VF52SEM5GQL7XEB`. The immutable
owner remains an explicit constructor argument and is intentionally not
hardcoded in this repository. Production deployment must verify that the
manager reports that same owner before constructing this contract.

## Development

The workspace pins Rust 1.91.1 and Soroban SDK 27.0.3.

```sh
make test
make build
```

No production deployment is recorded yet. Deployment must use the exact
source-attested release WASM and the migration repository's fail-closed
deployment runner.
