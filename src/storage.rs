//! Storage helpers. Every persistent write extends the entry's TTL.
// WAVE-ISSUE(contracts-5): a storage and TTL cost report would measure the entries written here.

use crate::types::{DataKey, Plan, Subscription, TTL_EXTEND_TO, TTL_THRESHOLD};
use crate::Error;
use soroban_sdk::Env;

fn extend(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

/// Read a counter. A missing counter means nothing has been created yet.
pub fn count(env: &Env, key: &DataKey) -> u64 {
    env.storage().persistent().get(key).unwrap_or(0)
}

/// Increment a counter and return the new value, which is the id of the new entry.
/// Ids start at 1 and are never reused because the counter only goes up.
pub fn next_id(env: &Env, key: &DataKey) -> Result<u64, Error> {
    let id = count(env, key).checked_add(1).ok_or(Error::Overflow)?;
    env.storage().persistent().set(key, &id);
    extend(env, key);
    Ok(id)
}

pub fn get_plan(env: &Env, plan_id: u64) -> Result<Plan, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Plan(plan_id))
        .ok_or(Error::PlanNotFound)
}

pub fn set_plan(env: &Env, plan_id: u64, plan: &Plan) {
    let key = DataKey::Plan(plan_id);
    env.storage().persistent().set(&key, plan);
    extend(env, &key);
}

/// Extend a plan's TTL without changing it. Used when something depends on the plan staying alive.
pub fn bump_plan(env: &Env, plan_id: u64) {
    extend(env, &DataKey::Plan(plan_id));
}

pub fn get_sub(env: &Env, sub_id: u64) -> Result<Subscription, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Sub(sub_id))
        .ok_or(Error::SubNotFound)
}

pub fn set_sub(env: &Env, sub_id: u64, sub: &Subscription) {
    let key = DataKey::Sub(sub_id);
    env.storage().persistent().set(&key, sub);
    extend(env, &key);
}
