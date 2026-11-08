#![no_std]

#[cfg(any(test, feature = "testutils"))]
extern crate std;

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
pub use types::{BackstopTier, ClaimAndFundDetails, ClaimState, Config, Totals};

pub const VESTING_DURATION_SECONDS: u64 = 360 * 24 * 60 * 60;
