//! Events. Every state change emits one. Each carries the ids and addresses needed to rebuild
//! state off-chain. The first topic is the snake-case struct name.

use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanCreated {
    #[topic]
    pub plan_id: u64,
    #[topic]
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub period_secs: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanDeactivated {
    #[topic]
    pub plan_id: u64,
    #[topic]
    pub merchant: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subscribed {
    #[topic]
    pub sub_id: u64,
    #[topic]
    pub plan_id: u64,
    #[topic]
    pub subscriber: Address,
    /// Amount paid now for the first period.
    pub amount: i128,
    pub next_charge_ts: u64,
}
