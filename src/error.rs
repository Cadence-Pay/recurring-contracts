//! Contract errors.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Amount is zero or negative.
    InvalidAmount = 1,
    /// Period is outside MIN_PERIOD_SECS..=MAX_PERIOD_SECS.
    InvalidPeriod = 2,
    PlanNotFound = 3,
    /// The plan was deactivated and accepts no new subscriptions.
    PlanInactive = 4,
    SubNotFound = 5,
    /// The subscription is cancelled.
    NotActive = 6,
    /// The next charge time has not been reached.
    NotDue = 7,
    /// The caller may not perform this action.
    NotAuthorized = 8,
    /// An arithmetic operation overflowed.
    Overflow = 9,
    /// The token transfer failed: balance too low, allowance too low or expired, or the token
    /// refused. Without this, the token's own error code would pass through and could be read as
    /// one of the codes above (for example the token's code 9 looks like `Overflow`).
    PaymentFailed = 10,
}
