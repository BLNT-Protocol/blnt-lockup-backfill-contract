#![no_std]

#[cfg(any(test, feature = "testutils"))]
extern crate std;

mod allocations;
mod clients;
mod contract;
mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

pub use contract::{BlntLockupBackfillContract, BlntLockupBackfillContractClient};
pub use errors::LockupBackfillError;
pub use types::{BackstopTier, ClaimState, Config, RouteResult, Totals};

pub const VESTING_DURATION_SECONDS: u64 = 360 * 24 * 60 * 60;
pub const BENEFICIARY_COUNT: u32 = 4;
pub const TOTAL_ALLOCATION: i128 = 37_442_405_803_143;
