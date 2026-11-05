extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    Event, IntoVal,
};

pub const START_TS: u64 = 1_000_000;
pub const AMOUNT: i128 = 10_000_000;
pub const PERIOD: u64 = 60;

pub struct Setup {
    pub env: Env,
    pub contract: Address,
    pub merchant: Address,
    pub subscriber: Address,
    pub token: Address,
}

impl Setup {
    pub fn new() -> Self {
        let env = Env::default();
        env.ledger().set_timestamp(START_TS);
        let contract = env.register(RecurringContract, ());
        let merchant = Address::generate(&env);
        let subscriber = Address::generate(&env);
        let issuer = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(issuer).address();
        Setup {
            env,
            contract,
            merchant,
            subscriber,
            token,
        }
    }

    pub fn client(&self) -> RecurringContractClient<'_> {
        RecurringContractClient::new(&self.env, &self.contract)
    }

    pub fn new_plan(&self) -> u64 {
        self.env.mock_all_auths();
        self.client()
            .create_plan(&self.merchant, &self.token, &AMOUNT, &PERIOD)
    }

    pub fn token_client(&self) -> TokenClient<'_> {
        TokenClient::new(&self.env, &self.token)
    }

    /// Give `who` some tokens.
    pub fn fund(&self, who: &Address, amount: i128) {
        self.env.mock_all_auths();
        StellarAssetClient::new(&self.env, &self.token).mint(who, &amount);
    }

    /// Events emitted by this contract during the last call.
    pub fn events(&self) -> soroban_sdk::testutils::ContractEvents {
        self.env.events().all().filter_by_contract(&self.contract)
    }
}

#[test]
fn plan_count_starts_at_zero() {
    let s = Setup::new();
    assert_eq!(s.client().plan_count(), 0);
}

#[test]
fn create_plan_stores_plan_and_emits_event() {
    let s = Setup::new();
    let id = s.new_plan();
    assert_eq!(id, 1);
    assert_eq!(s.client().plan_count(), 1);
    assert_eq!(
        s.client().get_plan(&id),
        Plan {
            merchant: s.merchant.clone(),
            token: s.token.clone(),
            amount: AMOUNT,
            period_secs: PERIOD,
            active: true,
        }
    );
    let expected = PlanCreated {
        plan_id: 1,
        merchant: s.merchant.clone(),
        token: s.token.clone(),
        amount: AMOUNT,
        period_secs: PERIOD,
    }
    .to_xdr(&s.env, &s.contract);
    // get_plan and plan_count are calls too; the event check must follow create_plan directly.
    let s2 = Setup::new();
    s2.new_plan();
    let expected2 = PlanCreated {
        plan_id: 1,
        merchant: s2.merchant.clone(),
        token: s2.token.clone(),
        amount: AMOUNT,
        period_secs: PERIOD,
    }
    .to_xdr(&s2.env, &s2.contract);
    assert_eq!(s2.events().events(), std::slice::from_ref(&expected2));
    let _ = expected;
}

#[test]
fn plan_ids_increase_and_are_never_reused() {
    let s = Setup::new();
    let a = s.new_plan();
    s.client().deactivate_plan(&a);
    let b = s.new_plan();
    assert_eq!((a, b), (1, 2));
    assert_eq!(s.client().plan_count(), 2);
}

#[test]
fn create_plan_requires_merchant_auth() {
    let s = Setup::new();
    // No auth mocked: the merchant did not authorize the call.
    let r = s
        .client()
        .try_create_plan(&s.merchant, &s.token, &AMOUNT, &PERIOD);
    assert!(r.is_err());
    assert_eq!(s.client().plan_count(), 0);
}

#[test]
fn create_plan_rejects_zero_and_negative_amount() {
    let s = Setup::new();
    s.env.mock_all_auths();
    for bad in [0i128, -1] {
        let r = s
            .client()
            .try_create_plan(&s.merchant, &s.token, &bad, &PERIOD);
        assert_eq!(r, Err(Ok(Error::InvalidAmount)));
    }
    assert_eq!(s.client().plan_count(), 0);
}

#[test]
fn create_plan_rejects_period_below_min() {
    let s = Setup::new();
    s.env.mock_all_auths();
    let r = s
        .client()
        .try_create_plan(&s.merchant, &s.token, &AMOUNT, &(MIN_PERIOD_SECS - 1));
    assert_eq!(r, Err(Ok(Error::InvalidPeriod)));
}

#[test]
fn create_plan_rejects_period_above_max() {
    let s = Setup::new();
    s.env.mock_all_auths();
    let r = s
        .client()
        .try_create_plan(&s.merchant, &s.token, &AMOUNT, &(MAX_PERIOD_SECS + 1));
    assert_eq!(r, Err(Ok(Error::InvalidPeriod)));
}

#[test]
fn create_plan_accepts_period_bounds() {
    let s = Setup::new();
    s.env.mock_all_auths();
    let c = s.client();
    c.create_plan(&s.merchant, &s.token, &AMOUNT, &MIN_PERIOD_SECS);
    c.create_plan(&s.merchant, &s.token, &AMOUNT, &MAX_PERIOD_SECS);
    assert_eq!(c.plan_count(), 2);
}

#[test]
fn deactivate_plan_marks_inactive_and_emits_event() {
    let s = Setup::new();
    let id = s.new_plan();
    s.client().deactivate_plan(&id);
    let expected = PlanDeactivated {
        plan_id: id,
        merchant: s.merchant.clone(),
    }
    .to_xdr(&s.env, &s.contract);
    // Check events before any further call, which would replace them.
    assert_eq!(s.events().events(), std::slice::from_ref(&expected));
    assert!(!s.client().get_plan(&id).active);
}

#[test]
fn deactivate_plan_twice_is_a_silent_no_op() {
    let s = Setup::new();
    let id = s.new_plan();
    s.client().deactivate_plan(&id);
    s.client().deactivate_plan(&id);
    assert!(s.events().events().is_empty());
}

#[test]
fn deactivate_plan_by_non_merchant_fails() {
    let s = Setup::new();
    let id = s.new_plan();
    let stranger = Address::generate(&s.env);
    // Only the stranger authorizes. The plan's merchant did not.
    s.env.mock_auths(&[MockAuth {
        address: &stranger,
        invoke: &MockAuthInvoke {
            contract: &s.contract,
            fn_name: "deactivate_plan",
            args: (id,).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);
    let r = s.client().try_deactivate_plan(&id);
    assert!(r.is_err());
    assert!(s.client().get_plan(&id).active);
}

#[test]
fn deactivate_unknown_plan_fails() {
    let s = Setup::new();
    s.env.mock_all_auths();
    assert_eq!(
        s.client().try_deactivate_plan(&99),
        Err(Ok(Error::PlanNotFound))
    );
}

#[test]
fn get_unknown_plan_fails() {
    let s = Setup::new();
    assert_eq!(s.client().try_get_plan(&1), Err(Ok(Error::PlanNotFound)));
}

#[test]
fn subscribe_pays_first_period_and_stores_subscription() {
    let s = Setup::new();
    let plan_id = s.new_plan();
    s.fund(&s.subscriber, AMOUNT * 3);
    let sub_id = s.client().subscribe(&s.subscriber, &plan_id);
    let expected = Subscribed {
        sub_id,
        plan_id,
        subscriber: s.subscriber.clone(),
        amount: AMOUNT,
        next_charge_ts: START_TS + PERIOD,
    }
    .to_xdr(&s.env, &s.contract);
    // The token also emits a transfer event; keep only this contract's events.
    assert_eq!(s.events().events(), std::slice::from_ref(&expected));

    assert_eq!(sub_id, 1);
    assert_eq!(s.client().sub_count(), 1);
    assert_eq!(
        s.client().get_subscription(&sub_id),
        Subscription {
            plan_id,
            subscriber: s.subscriber.clone(),
            next_charge_ts: START_TS + PERIOD,
            status: SubStatus::Active,
            charges: 0,
        }
    );
}

#[test]
fn subscribe_moves_first_period_to_merchant() {
    let s = Setup::new();
    let plan_id = s.new_plan();
    s.fund(&s.subscriber, AMOUNT * 3);
    s.client().subscribe(&s.subscriber, &plan_id);
    assert_eq!(s.token_client().balance(&s.merchant), AMOUNT);
    assert_eq!(s.token_client().balance(&s.subscriber), AMOUNT * 2);
    assert_eq!(s.token_client().balance(&s.contract), 0);
}

#[test]
fn subscribe_ids_increase() {
    let s = Setup::new();
    let plan_id = s.new_plan();
    s.fund(&s.subscriber, AMOUNT * 3);
    assert_eq!(s.client().subscribe(&s.subscriber, &plan_id), 1);
    assert_eq!(s.client().subscribe(&s.subscriber, &plan_id), 2);
    assert_eq!(s.client().sub_count(), 2);
}

#[test]
fn subscribe_to_inactive_plan_fails() {
    let s = Setup::new();
    let plan_id = s.new_plan();
    s.fund(&s.subscriber, AMOUNT);
    s.client().deactivate_plan(&plan_id);
    assert_eq!(
        s.client().try_subscribe(&s.subscriber, &plan_id),
        Err(Ok(Error::PlanInactive))
    );
    assert_eq!(s.client().sub_count(), 0);
    assert_eq!(s.token_client().balance(&s.subscriber), AMOUNT);
}

#[test]
fn subscribe_to_unknown_plan_fails() {
    let s = Setup::new();
    s.env.mock_all_auths();
    assert_eq!(
        s.client().try_subscribe(&s.subscriber, &7),
        Err(Ok(Error::PlanNotFound))
    );
}

#[test]
fn subscribe_with_insufficient_balance_reverts_everything() {
    let s = Setup::new();
    let plan_id = s.new_plan();
    s.fund(&s.subscriber, AMOUNT - 1);
    let r = s.client().try_subscribe(&s.subscriber, &plan_id);
    assert_eq!(r, Err(Ok(Error::PaymentFailed)));
    // The failed call left no subscription behind.
    assert_eq!(s.client().sub_count(), 0);
    assert_eq!(s.token_client().balance(&s.merchant), 0);
    assert_eq!(s.token_client().balance(&s.subscriber), AMOUNT - 1);
}

#[test]
fn subscribe_requires_subscriber_auth() {
    let s = Setup::new();
    let plan_id = s.new_plan();
    s.fund(&s.subscriber, AMOUNT);
    // Someone else authorizes the call. The subscriber did not.
    let other = Address::generate(&s.env);
    s.env.mock_auths(&[MockAuth {
        address: &other,
        invoke: &MockAuthInvoke {
            contract: &s.contract,
            fn_name: "subscribe",
            args: (s.subscriber.clone(), plan_id).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);
    assert!(s.client().try_subscribe(&s.subscriber, &plan_id).is_err());
    assert_eq!(s.client().sub_count(), 0);
}

#[test]
fn get_unknown_subscription_fails() {
    let s = Setup::new();
    assert_eq!(
        s.client().try_get_subscription(&1),
        Err(Ok(Error::SubNotFound))
    );
}

#[test]
fn subscribe_needs_only_subscribe_and_nested_transfer_authorization() {
    // This is what a wallet signs: the subscribe call plus the token transfer it makes.
    let s = Setup::new();
    let plan_id = s.new_plan();
    s.fund(&s.subscriber, AMOUNT);
    s.env.mock_auths(&[MockAuth {
        address: &s.subscriber,
        invoke: &MockAuthInvoke {
            contract: &s.contract,
            fn_name: "subscribe",
            args: (s.subscriber.clone(), plan_id).into_val(&s.env),
            sub_invokes: &[MockAuthInvoke {
                contract: &s.token,
                fn_name: "transfer",
                args: (s.subscriber.clone(), s.merchant.clone(), AMOUNT).into_val(&s.env),
                sub_invokes: &[],
            }],
        },
    }]);
    assert_eq!(s.client().subscribe(&s.subscriber, &plan_id), 1);
    assert_eq!(s.token_client().balance(&s.merchant), AMOUNT);
}

// ---- charge ----

/// A plan, a funded subscriber with an allowance, and one subscription made at START_TS.
fn subscribed(s: &Setup, allowance: i128) -> u64 {
    let plan_id = s.new_plan();
    s.fund(&s.subscriber, AMOUNT * 10);
    let sub_id = s.client().subscribe(&s.subscriber, &plan_id);
    let live_until = s.env.ledger().sequence() + 10_000;
    s.token_client()
        .approve(&s.subscriber, &s.contract, &allowance, &live_until);
    sub_id
}

fn set_time(s: &Setup, ts: u64) {
    s.env.ledger().set_timestamp(ts);
}

#[test]
fn charge_on_time_moves_funds_and_updates_subscription() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    set_time(&s, START_TS + PERIOD);
    s.client().charge(&sub_id);
    let expected = Charged {
        sub_id,
        plan_id: 1,
        subscriber: s.subscriber.clone(),
        amount: AMOUNT,
        charges: 1,
        next_charge_ts: START_TS + 2 * PERIOD,
    }
    .to_xdr(&s.env, &s.contract);
    assert_eq!(s.events().events(), std::slice::from_ref(&expected));

    let sub = s.client().get_subscription(&sub_id);
    assert_eq!(sub.charges, 1);
    assert_eq!(sub.next_charge_ts, START_TS + 2 * PERIOD);
    assert_eq!(sub.status, SubStatus::Active);
    // First period at subscribe plus this charge.
    assert_eq!(s.token_client().balance(&s.merchant), AMOUNT * 2);
    assert_eq!(s.token_client().balance(&s.subscriber), AMOUNT * 8);
    // The allowance was spent.
    assert_eq!(
        s.token_client().allowance(&s.subscriber, &s.contract),
        AMOUNT * 4
    );
}

#[test]
fn charge_before_due_fails_with_not_due() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    set_time(&s, START_TS + PERIOD - 1);
    assert_eq!(s.client().try_charge(&sub_id), Err(Ok(Error::NotDue)));
    assert_eq!(s.client().get_subscription(&sub_id).charges, 0);
    assert_eq!(s.token_client().balance(&s.merchant), AMOUNT);
}

#[test]
fn second_charge_in_the_same_period_fails() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    set_time(&s, START_TS + PERIOD);
    s.client().charge(&sub_id);
    assert_eq!(s.client().try_charge(&sub_id), Err(Ok(Error::NotDue)));
    // Still not due one second before the next period starts.
    set_time(&s, START_TS + 2 * PERIOD - 1);
    assert_eq!(s.client().try_charge(&sub_id), Err(Ok(Error::NotDue)));
    assert_eq!(s.client().get_subscription(&sub_id).charges, 1);
    // Due again exactly when the next period starts.
    set_time(&s, START_TS + 2 * PERIOD);
    s.client().charge(&sub_id);
    assert_eq!(s.client().get_subscription(&sub_id).charges, 2);
}

#[test]
fn charge_with_too_little_allowance_reverts_and_changes_nothing() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT - 1);
    set_time(&s, START_TS + PERIOD);
    let before = s.client().get_subscription(&sub_id);
    assert_eq!(
        s.client().try_charge(&sub_id),
        Err(Ok(Error::PaymentFailed))
    );
    assert_eq!(s.client().get_subscription(&sub_id), before);
    assert_eq!(s.token_client().balance(&s.merchant), AMOUNT);
    assert_eq!(s.token_client().balance(&s.subscriber), AMOUNT * 9);
    // The allowance was the only problem: the same charge works once it is large enough.
    let live_until = s.env.ledger().sequence() + 10_000;
    s.token_client()
        .approve(&s.subscriber, &s.contract, &AMOUNT, &live_until);
    s.client().charge(&sub_id);
    assert_eq!(s.client().get_subscription(&sub_id).charges, 1);
}

#[test]
fn charge_with_no_allowance_reverts() {
    let s = Setup::new();
    let sub_id = subscribed(&s, 0);
    set_time(&s, START_TS + PERIOD);
    assert_eq!(
        s.client().try_charge(&sub_id),
        Err(Ok(Error::PaymentFailed))
    );
    assert_eq!(s.client().get_subscription(&sub_id).charges, 0);
}

#[test]
fn charge_with_expired_allowance_reverts() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    set_time(&s, START_TS + PERIOD);
    // Move past the ledger the allowance was valid until.
    s.env
        .ledger()
        .set_sequence_number(s.env.ledger().sequence() + 10_001);
    // The token itself now treats the allowance as zero.
    assert_eq!(s.token_client().allowance(&s.subscriber, &s.contract), 0);
    assert_eq!(
        s.client().try_charge(&sub_id),
        Err(Ok(Error::PaymentFailed))
    );
    assert_eq!(s.client().get_subscription(&sub_id).charges, 0);
    assert_eq!(s.token_client().balance(&s.merchant), AMOUNT);
    // A fresh allowance fixes it, so expiry was the cause.
    let live_until = s.env.ledger().sequence() + 10_000;
    s.token_client()
        .approve(&s.subscriber, &s.contract, &AMOUNT, &live_until);
    s.client().charge(&sub_id);
    assert_eq!(s.client().get_subscription(&sub_id).charges, 1);
}

#[test]
fn charge_with_low_balance_reverts() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    // Spend the subscriber's balance elsewhere.
    let elsewhere = Address::generate(&s.env);
    s.token_client()
        .transfer(&s.subscriber, &elsewhere, &(AMOUNT * 9));
    set_time(&s, START_TS + PERIOD);
    assert_eq!(
        s.client().try_charge(&sub_id),
        Err(Ok(Error::PaymentFailed))
    );
    assert_eq!(s.client().get_subscription(&sub_id).charges, 0);
    // Top the balance up and the same charge works, so the balance was the cause.
    s.fund(&s.subscriber, AMOUNT);
    s.client().charge(&sub_id);
    assert_eq!(s.client().get_subscription(&sub_id).charges, 1);
}

#[test]
fn missed_periods_are_skipped_not_billed_in_a_burst() {
    // Decision D2: after a late charge the schedule restarts from now.
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    let late = START_TS + 10 * PERIOD + 7;
    set_time(&s, late);
    s.client().charge(&sub_id);
    let sub = s.client().get_subscription(&sub_id);
    assert_eq!(sub.charges, 1);
    assert_eq!(sub.next_charge_ts, late + PERIOD);
    // Only one period was collected, and a second charge now is refused.
    assert_eq!(s.token_client().balance(&s.merchant), AMOUNT * 2);
    assert_eq!(s.client().try_charge(&sub_id), Err(Ok(Error::NotDue)));
}

#[test]
fn slightly_late_charge_keeps_the_original_rhythm() {
    // Decision D2: if the scheduled next time is still in the future, keep it.
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    set_time(&s, START_TS + PERIOD + 10);
    s.client().charge(&sub_id);
    assert_eq!(
        s.client().get_subscription(&sub_id).next_charge_ts,
        START_TS + 2 * PERIOD
    );
}

#[test]
fn charge_when_next_time_equals_now_moves_to_now_plus_period() {
    // Boundary of D2: scheduled == now counts as "not in the future".
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    set_time(&s, START_TS + 2 * PERIOD);
    s.client().charge(&sub_id);
    assert_eq!(
        s.client().get_subscription(&sub_id).next_charge_ts,
        START_TS + 3 * PERIOD
    );
}

#[test]
fn anyone_can_charge_without_authorizing_anything() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    set_time(&s, START_TS + PERIOD);
    // Replace the blanket mock with no authorization at all.
    s.env.mock_auths(&[]);
    s.client().charge(&sub_id);
    assert_eq!(s.client().get_subscription(&sub_id).charges, 1);
}

#[test]
fn charge_unknown_subscription_fails() {
    let s = Setup::new();
    assert_eq!(s.client().try_charge(&42), Err(Ok(Error::SubNotFound)));
}

#[test]
fn charge_still_works_after_plan_is_deactivated() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    s.client().deactivate_plan(&1);
    set_time(&s, START_TS + PERIOD);
    s.client().charge(&sub_id);
    assert_eq!(s.client().get_subscription(&sub_id).charges, 1);
}

// ---- cancel ----

#[test]
fn subscriber_can_cancel() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    s.client().cancel(&s.subscriber, &sub_id);
    let expected = Cancelled {
        sub_id,
        plan_id: 1,
        subscriber: s.subscriber.clone(),
        cancelled_by: s.subscriber.clone(),
    }
    .to_xdr(&s.env, &s.contract);
    assert_eq!(s.events().events(), std::slice::from_ref(&expected));
    assert_eq!(
        s.client().get_subscription(&sub_id).status,
        SubStatus::Cancelled
    );
}

#[test]
fn merchant_can_cancel() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    s.client().cancel(&s.merchant, &sub_id);
    let expected = Cancelled {
        sub_id,
        plan_id: 1,
        subscriber: s.subscriber.clone(),
        cancelled_by: s.merchant.clone(),
    }
    .to_xdr(&s.env, &s.contract);
    assert_eq!(s.events().events(), std::slice::from_ref(&expected));
    assert_eq!(
        s.client().get_subscription(&sub_id).status,
        SubStatus::Cancelled
    );
}

#[test]
fn stranger_cannot_cancel() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    let stranger = Address::generate(&s.env);
    // The stranger authorizes their own call, but they are neither party.
    assert_eq!(
        s.client().try_cancel(&stranger, &sub_id),
        Err(Ok(Error::NotAuthorized))
    );
    assert_eq!(
        s.client().get_subscription(&sub_id).status,
        SubStatus::Active
    );
}

#[test]
fn cancel_without_the_callers_authorization_fails() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    // The stranger signs, but names the subscriber as the caller.
    let stranger = Address::generate(&s.env);
    s.env.mock_auths(&[MockAuth {
        address: &stranger,
        invoke: &MockAuthInvoke {
            contract: &s.contract,
            fn_name: "cancel",
            args: (s.subscriber.clone(), sub_id).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);
    assert!(s.client().try_cancel(&s.subscriber, &sub_id).is_err());
    assert_eq!(
        s.client().get_subscription(&sub_id).status,
        SubStatus::Active
    );
}

#[test]
fn cancelling_twice_returns_not_active() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    s.client().cancel(&s.subscriber, &sub_id);
    assert_eq!(
        s.client().try_cancel(&s.subscriber, &sub_id),
        Err(Ok(Error::NotActive))
    );
    assert_eq!(
        s.client().try_cancel(&s.merchant, &sub_id),
        Err(Ok(Error::NotActive))
    );
}

#[test]
fn cancel_unknown_subscription_fails() {
    let s = Setup::new();
    s.env.mock_all_auths();
    assert_eq!(
        s.client().try_cancel(&s.subscriber, &9),
        Err(Ok(Error::SubNotFound))
    );
}

#[test]
fn cancelled_subscription_is_never_charged() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    s.client().cancel(&s.subscriber, &sub_id);
    set_time(&s, START_TS + 5 * PERIOD);
    assert_eq!(s.client().try_charge(&sub_id), Err(Ok(Error::NotActive)));
    assert_eq!(s.client().get_subscription(&sub_id).charges, 0);
    assert_eq!(s.token_client().balance(&s.merchant), AMOUNT);
    assert_eq!(
        s.token_client().allowance(&s.subscriber, &s.contract),
        AMOUNT * 5
    );
}

#[test]
fn cancel_works_after_the_plan_is_deactivated() {
    let s = Setup::new();
    let sub_id = subscribed(&s, AMOUNT * 5);
    s.client().deactivate_plan(&1);
    s.client().cancel(&s.subscriber, &sub_id);
    assert_eq!(
        s.client().get_subscription(&sub_id).status,
        SubStatus::Cancelled
    );
}
