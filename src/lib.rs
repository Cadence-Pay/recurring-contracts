//! Stellar Recurring: pull-based subscription billing.
//!
//! A merchant creates a plan. A subscriber pays the first period at `subscribe` and approves a
//! token allowance. Anyone may then call `charge` once per period. This is not payment streaming.
//!
//! Task 1.2: plan functions are implemented. `subscribe`, `charge` and `cancel` are still stubs.
#![no_std]

mod error;
mod events;
mod storage;
mod types;

pub use error::Error;
pub use events::{PlanCreated, PlanDeactivated};
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
    ///
    /// Fails with `InvalidAmount` if `amount <= 0` and with `InvalidPeriod` if `period_secs` is
    /// outside `MIN_PERIOD_SECS..=MAX_PERIOD_SECS`.
    pub fn create_plan(
        env: Env,
        merchant: Address,
        token: Address,
        amount: i128,
        period_secs: u64,
    ) -> Result<u64, Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if !(MIN_PERIOD_SECS..=MAX_PERIOD_SECS).contains(&period_secs) {
            return Err(Error::InvalidPeriod);
        }
        merchant.require_auth();

        let plan_id = storage::next_id(&env, &DataKey::PlanCount)?;
        let plan = Plan {
            merchant: merchant.clone(),
            token: token.clone(),
            amount,
            period_secs,
            active: true,
        };
        storage::set_plan(&env, plan_id, &plan);
        PlanCreated {
            plan_id,
            merchant,
            token,
            amount,
            period_secs,
        }
        .publish(&env);
        Ok(plan_id)
    }

    /// Stop accepting new subscriptions to a plan. Requires the plan merchant's authorization.
    /// Existing subscriptions keep working. Deactivating an inactive plan does nothing.
    pub fn deactivate_plan(env: Env, plan_id: u64) -> Result<(), Error> {
        let mut plan = storage::get_plan(&env, plan_id)?;
        plan.merchant.require_auth();
        if !plan.active {
            return Ok(());
        }
        plan.active = false;
        storage::set_plan(&env, plan_id, &plan);
        PlanDeactivated {
            plan_id,
            merchant: plan.merchant,
        }
        .publish(&env);
        Ok(())
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
    pub fn get_plan(env: Env, plan_id: u64) -> Result<Plan, Error> {
        storage::get_plan(&env, plan_id)
    }

    /// Read a subscription.
    pub fn get_subscription(_env: Env, _sub_id: u64) -> Result<Subscription, Error> {
        Err(Error::SubNotFound)
    }

    /// Number of plans created so far. Ids run from 1 to this value.
    pub fn plan_count(env: Env) -> u64 {
        storage::count(&env, &DataKey::PlanCount)
    }

    /// Number of subscriptions created so far. Ids run from 1 to this value.
    pub fn sub_count(_env: Env) -> u64 {
        0
    }
}

#[cfg(test)]
mod test;
