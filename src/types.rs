//! Data types, storage keys and constants.

use soroban_sdk::{contracttype, Address};

/// Shortest allowed billing period in seconds. Kept at 60 so rehearsals are quick.
/// Revisit before a mainnet release.
pub const MIN_PERIOD_SECS: u64 = 60;
/// Longest allowed billing period in seconds (365 days).
pub const MAX_PERIOD_SECS: u64 = 365 * 24 * 60 * 60;

/// Ledgers per day, assuming a 5 second close time (verified on testnet, see docs/RESEARCH.md).
pub const LEDGERS_PER_DAY: u32 = 17_280;
/// Extend an entry when its remaining life drops below this many ledgers (30 days).
pub const TTL_THRESHOLD: u32 = 30 * LEDGERS_PER_DAY;
/// Extend an entry to this many ledgers of remaining life (90 days).
/// Must stay below the network's maximum entry TTL (3,110,400 ledgers on testnet).
pub const TTL_EXTEND_TO: u32 = 90 * LEDGERS_PER_DAY;

/// A billing plan created by a merchant.
// WAVE-ISSUE(contracts-3): plan changes with proration plug in here. The MVP has no plan change.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plan {
    /// Receives every payment and may deactivate the plan.
    pub merchant: Address,
    /// SEP-41 token contract used for payments.
    pub token: Address,
    /// Amount per period, in the token's smallest unit.
    pub amount: i128,
    /// Length of one billing period in seconds.
    pub period_secs: u64,
    /// False once deactivated. No new subscriptions, existing ones keep working.
    pub active: bool,
}

/// Lifecycle state of a subscription.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubStatus {
    Active,
    Cancelled,
}

/// A subscriber's subscription to a plan.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subscription {
    pub plan_id: u64,
    pub subscriber: Address,
    /// Ledger timestamp (seconds) from which the next charge is allowed.
    pub next_charge_ts: u64,
    pub status: SubStatus,
    /// Number of successful charges after the first payment made at subscribe.
    pub charges: u32,
}

/// Storage keys. One persistent entry per id, no lists.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    PlanCount,
    SubCount,
    Plan(u64),
    Sub(u64),
}
