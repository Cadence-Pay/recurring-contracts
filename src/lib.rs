//! Stellar Recurring: pull-based subscription billing.
//!
//! This is the Phase 0 skeleton. The real contract is built in Phase 1.
#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct RecurringContract;

#[contractimpl]
impl RecurringContract {
    /// Number of plans created so far. The skeleton has no storage yet, so it is always 0.
    pub fn plan_count(_env: Env) -> u64 {
        0
    }
}

#[cfg(test)]
mod test;
