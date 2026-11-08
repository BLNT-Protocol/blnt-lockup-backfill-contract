use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LockupBackfillError {
    InvalidConfig = 1500,
    InvalidToken = 1501,
    InvalidAllocation = 1502,
    DuplicateBeneficiary = 1503,
    InvalidVesting = 1505,
    InvalidManager = 1506,
    NoClaim = 1510,
    NothingClaimable = 1511,
    InvalidAmount = 1512,
    Overfunded = 1513,
    BalanceMismatch = 1514,
    ReentrantCall = 1515,
    Overflow = 1516,
    ClaimsClosed = 1520,
}
