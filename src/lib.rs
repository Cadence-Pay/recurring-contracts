//! Stellar Recurring: pull-based subscription billing.
//!
//! A merchant creates a plan. A subscriber pays the first period at `subscribe` and approves a
//! token allowance. Anyone may then call `charge` once per period. This is not payment streaming.
//!
//! Task 1.1: the public functions below are stubs. They are implemented in tasks 1.2 to 1.5.
#![no_std]

mod error;
mod types;

pub use error::Error;
pub use types::{
    DataKey, Plan, SubStatus, Subscription, LEDGERS_PER_DAY, MAX_PERIOD_SECS, MIN_PERIOD_SECS,
    TTL_EXTEND_TO, TTL_THRESHOLD,
};

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct RecurringContract;

#[contractimpl]
impl RecurringContract {
    /// Create a plan. Requires the merchant's authorization. Returns the new plan id.
    pub fn create_plan(
        _env: Env,
        _merchant: Address,
        _token: Address,
        _amount: i128,
        _period_secs: u64,
    ) -> Result<u64, Error> {
        Err(Error::PlanNotFound)
    }

    /// Stop accepting new subscriptions to a plan. Requires the plan merchant's authorization.
    pub fn deactivate_plan(_env: Env, _plan_id: u64) -> Result<(), Error> {
        Err(Error::PlanNotFound)
    }

    /// Subscribe and pay the first period now. Requires the subscriber's authorization.
    /// Returns the new subscription id.
    pub fn subscribe(_env: Env, _subscriber: Address, _plan_id: u64) -> Result<u64, Error> {
        Err(Error::PlanNotFound)
    }

    /// Charge one period. Permissionless: any account may call it once the subscription is due.
    pub fn charge(_env: Env, _sub_id: u64) -> Result<(), Error> {
        Err(Error::SubNotFound)
    }

    /// Cancel a subscription. Only the subscriber or the plan's merchant may do so.
    pub fn cancel(_env: Env, _caller: Address, _sub_id: u64) -> Result<(), Error> {
        Err(Error::SubNotFound)
    }

    /// Read a plan.
    pub fn get_plan(_env: Env, _plan_id: u64) -> Result<Plan, Error> {
        Err(Error::PlanNotFound)
    }

    /// Read a subscription.
    pub fn get_subscription(_env: Env, _sub_id: u64) -> Result<Subscription, Error> {
        Err(Error::SubNotFound)
    }

    /// Number of plans created so far. Ids run from 1 to this value.
    pub fn plan_count(_env: Env) -> u64 {
        0
    }

    /// Number of subscriptions created so far. Ids run from 1 to this value.
    pub fn sub_count(_env: Env) -> u64 {
        0
    }
}

#[cfg(test)]
mod test;
