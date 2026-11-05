//! Stellar Recurring: pull-based subscription billing.
//!
//! A merchant creates a plan. A subscriber pays the first period at `subscribe` and approves a
//! token allowance. Anyone may then call `charge` once per period. This is not payment streaming.
//!
//! Task 1.4: plan functions, `subscribe` and `charge` are implemented. `cancel` is still a stub.
#![no_std]

mod error;
mod events;
mod storage;
mod types;

pub use error::Error;
pub use events::{Charged, PlanCreated, PlanDeactivated, Subscribed};
pub use types::{
    DataKey, Plan, SubStatus, Subscription, LEDGERS_PER_DAY, MAX_PERIOD_SECS, MIN_PERIOD_SECS,
    TTL_EXTEND_TO, TTL_THRESHOLD,
};

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env};

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
    ///
    /// The first period is paid with a direct `transfer` from the subscriber to the merchant, so
    /// no allowance is needed for it. The next charge is due one period from now. If the token
    /// transfer fails (for example the balance is too low) the whole call reverts.
    ///
    /// Fails with `PlanNotFound` or `PlanInactive` if the plan cannot take subscriptions.
    pub fn subscribe(env: Env, subscriber: Address, plan_id: u64) -> Result<u64, Error> {
        subscriber.require_auth();
        let plan = storage::get_plan(&env, plan_id)?;
        if !plan.active {
            return Err(Error::PlanInactive);
        }
        let next_charge_ts = env
            .ledger()
            .timestamp()
            .checked_add(plan.period_secs)
            .ok_or(Error::Overflow)?;

        let sub_id = storage::next_id(&env, &DataKey::SubCount)?;
        let sub = Subscription {
            plan_id,
            subscriber: subscriber.clone(),
            next_charge_ts,
            status: SubStatus::Active,
            charges: 0,
        };
        storage::set_sub(&env, sub_id, &sub);
        storage::bump_plan(&env, plan_id);

        // WAVE-ISSUE(contracts-2): trial periods plug in here. The MVP always charges the first
        // period at subscribe.
        TokenClient::new(&env, &plan.token).transfer(&subscriber, &plan.merchant, &plan.amount);

        Subscribed {
            sub_id,
            plan_id,
            subscriber,
            amount: plan.amount,
            next_charge_ts,
        }
        .publish(&env);
        Ok(sub_id)
    }

    /// Charge one period. Permissionless: any account may call it once the subscription is due.
    ///
    /// Requires an `Active` subscription (`NotActive` otherwise) and `now >= next_charge_ts`
    /// (`NotDue` otherwise). Moves the plan amount from the subscriber to the merchant with
    /// `transfer_from`, spending the allowance the subscriber gave this contract. The contract is
    /// the spender and the invoker, so the caller does not need to authorize anything.
    ///
    /// The next due time follows decision D2 (no back-billing): `next_charge_ts + period_secs`,
    /// or `now + period_secs` if that is not in the future. At most one charge happens per call.
    ///
    /// If the token call fails (no allowance, expired allowance, low balance) the whole call
    /// reverts and nothing changes.
    pub fn charge(env: Env, sub_id: u64) -> Result<(), Error> {
        let mut sub = storage::get_sub(&env, sub_id)?;
        if sub.status != SubStatus::Active {
            return Err(Error::NotActive);
        }
        let now = env.ledger().timestamp();
        if now < sub.next_charge_ts {
            return Err(Error::NotDue);
        }
        let plan = storage::get_plan(&env, sub.plan_id)?;

        let scheduled = sub
            .next_charge_ts
            .checked_add(plan.period_secs)
            .ok_or(Error::Overflow)?;
        sub.next_charge_ts = if scheduled <= now {
            now.checked_add(plan.period_secs).ok_or(Error::Overflow)?
        } else {
            scheduled
        };
        sub.charges = sub.charges.checked_add(1).ok_or(Error::Overflow)?;
        storage::set_sub(&env, sub_id, &sub);
        storage::bump_plan(&env, sub.plan_id);

        // WAVE-ISSUE(contracts-1): a failed-charge grace period plugs in here. In the MVP a
        // failing transfer_from reverts the whole call and no failure is recorded on-chain.
        TokenClient::new(&env, &plan.token).transfer_from(
            &env.current_contract_address(),
            &sub.subscriber,
            &plan.merchant,
            &plan.amount,
        );

        Charged {
            sub_id,
            plan_id: sub.plan_id,
            subscriber: sub.subscriber,
            amount: plan.amount,
            charges: sub.charges,
            next_charge_ts: sub.next_charge_ts,
        }
        .publish(&env);
        Ok(())
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
    pub fn get_subscription(env: Env, sub_id: u64) -> Result<Subscription, Error> {
        storage::get_sub(&env, sub_id)
    }

    /// Number of plans created so far. Ids run from 1 to this value.
    pub fn plan_count(env: Env) -> u64 {
        storage::count(&env, &DataKey::PlanCount)
    }

    /// Number of subscriptions created so far. Ids run from 1 to this value.
    pub fn sub_count(env: Env) -> u64 {
        storage::count(&env, &DataKey::SubCount)
    }
}

#[cfg(test)]
mod test;
