use super::*;
use soroban_sdk::Env;

#[test]
fn plan_count_starts_at_zero() {
    let env = Env::default();
    let id = env.register(RecurringContract, ());
    let client = RecurringContractClient::new(&env, &id);
    assert_eq!(client.plan_count(), 0);
}
