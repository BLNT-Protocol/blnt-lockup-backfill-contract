# Repository Instructions

These instructions apply to the entire repository.

## Mission

Implement and verify a minimal, immutable Soroban contract that accepts
voluntarily contributed BLNT and pays replacement backfill entitlements to the
verified owners of four immutable `blend-lockup v1.0.0` contracts.

The contract corrects an address-attribution problem without modifying the
deployed BLNT backfill contract, minting BLNT, assuming custody of issuer
authority, or changing any existing allocation. Keep the implementation
narrowly limited to funding, vesting, claiming, and read-only accounting.

## Normative source

Read `docs/SYSTEM_SPEC.md` completely before changing contract behavior. Its
`MUST`, `MUST NOT`, `SHOULD`, and `MAY` statements are acceptance criteria.

Evidence used to construct the immutable allocation table MUST remain
reproducible from these external sources:

- the deployed BLNT backfill contract and its 360-day contributor-grant
  vesting timestamps;
- the pre-incident Comet LP ownership snapshot;
- the `blend-comet-lp-remediation` claimant and lockup-owner evidence; and
- the deployed BLNT backfill allocation manifest.

Do not silently replace an owner, merge successor, allocation, token,
contract, timestamp, or evidence hash.

## Critical invariants

- Never mint BLNT or hold BLNT issuer/admin authority.
- Never mutate an allocation, beneficiary, vesting epoch, or binding after
  construction.
- Claims require authorization by the immutable beneficiary and pay only that
  beneficiary.
- Funding never accelerates vesting.
- Underfunding never erases entitlement: a claim pays the lesser of currently
  vested-and-unclaimed entitlement and available BLNT, leaving the remainder
  claimable later.
- A successful payout increments progress by exactly the transferred amount.
- Failed authorization, transfer, balance verification, or arithmetic MUST
  roll back all state changes.
- The contract MUST reject overfunding so no BLNT becomes stranded surplus.
- The atomic donor path MUST either complete the designated donor's canonical
  contributor-grant claim, the requested lockup contribution, both canonical
  Comet V1.1 LP joins, and both YieldBlent backstop deposits or complete none
  of them.
- Use checked integer arithmetic and seven-decimal token units. Never use
  floating-point arithmetic in contract code or allocation generation.
- Bound all maps and loops. Claim and funding entrypoints MUST be constant in
  the number of beneficiaries.
- Do not add an administrator, upgrade, sweep, recovery, governance, or
  beneficiary-mutation path.

## Development workflow

Before editing, inspect `git status` and preserve unrelated user work. For
contract changes:

1. Update the system specification first when behavior changes.
2. Add or update focused native tests.
3. Test authorization, vesting boundaries, rounding, partial funding,
   repeated claims, overfunding, the one-third manager split, both LP joins,
   both YieldBlent deposits, reentrancy, token-balance mismatches, arithmetic
   boundaries, and rollback.
4. Build optimized `wasm32v1-none` WASM with the pinned toolchain.
5. Verify source metadata and record the optimized WASM hash before any
   deployment proposal.

Deployment, publishing, tagging, funding, or other external state changes
require explicit user authorization. Never commit secrets, signing material,
private account configuration, generated target directories, or deployment
credentials.

## Repository hygiene

- Use `apply_patch` for authored file changes.
- Keep the workspace lockfile and release inputs committed.
- Preserve AGPL-3.0 notices once licensing is added.
- Do not describe the contract as audited or official unless that status is
  independently established.
- Prefer the smallest auditable ABI and implementation consistent with the
  system specification.
