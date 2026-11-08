# Repository Instructions

These instructions apply to the entire repository.

## Mission

Implement and verify a minimal, immutable Soroban contract that accepts
voluntarily contributed BLNT and pays replacement backfill entitlements to the
verified owners of four immutable `blend-lockup v1.0.0` contracts.

The contract corrects an address-attribution problem without modifying the
deployed BLNT backfill contract, minting BLNT, assuming custody of issuer
authority, or changing any existing allocation. Keep the implementation
narrowly limited to funding, vesting, claiming, donor-authorized claim closure
and refund, and read-only accounting.

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
- The contract MUST reject overfunding while claims are open. At any time, only
  the designated donor may atomically close claims and refund the contract's
  complete remaining BLNT custody to the donor address. Claims MUST remain open
  until that explicit call succeeds.
- During backfill the designated donor MUST be able to operate the backstop
  manager either as its owner or as its configured high-scope manager. While
  the donor can operate the contract, the atomic donor path MUST either
  complete the donor's canonical contributor-grant claim, requested lockup
  contribution, both canonical Comet V1.1 LP joins, and both YieldBlent
  deposits or complete none of them. If donor access is removed prematurely,
  the same path MUST claim and fund only the requested lockup amount while
  retaining the complete one-third manager allocation in the donor wallet.
- Use checked integer arithmetic and seven-decimal token units. Never use
  floating-point arithmetic in contract code or allocation generation.
- Bound all maps and loops. Claim and funding entrypoints MUST be constant in
  the number of beneficiaries.
- Do not add an administrator, upgrade, arbitrary recovery,
  governance, or beneficiary-mutation path. The only recovery path is the
  specified donor-authorized BLNT claim closure and refund.

## Development workflow

Before editing, inspect `git status` and preserve unrelated user work. For
contract changes:

1. Update the system specification first when behavior changes.
2. Add or update focused native tests.
3. Test authorization, vesting boundaries, rounding, partial funding,
   repeated claims, overfunding, the one-third manager split, both LP joins,
   both YieldBlent deposits, closure authorization,
   reentrancy, token-balance mismatches, arithmetic boundaries, and rollback.
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
