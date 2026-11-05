# Contract reference

**Audit status: this contract has not been audited.** Everything below describes what is implemented and covered by the test suite in `src/test.rs` (48 tests). The contract is not yet deployed (task 1.7).

Pull-based subscription billing. A merchant creates a plan. A subscriber pays the first period when subscribing and gives the contract a token allowance. After that, anyone can call `charge` once per period. This is not payment streaming: one discrete transfer per period.

Built with `soroban-sdk` 29.0.1. All times are ledger timestamps in seconds. All amounts are `i128` in the token's smallest unit. The token must follow SEP-41 (`transfer`, `transfer_from`, `approve`).

## Functions

| Function | Who may call | What it does |
|---|---|---|
| `create_plan(merchant, token, amount, period_secs) -> Result<u64, Error>` | `merchant` must authorize | Creates a plan and returns its id (from 1). |
| `deactivate_plan(plan_id) -> Result<(), Error>` | the plan's merchant must authorize | No new subscriptions. Existing ones keep working. Doing it twice is a silent no-op (no event). |
| `subscribe(subscriber, plan_id) -> Result<u64, Error>` | `subscriber` must authorize | Pays the first period now with `token.transfer(subscriber -> merchant)`. Next charge due at `now + period_secs`. Returns the subscription id (from 1). |
| `charge(sub_id) -> Result<(), Error>` | **anyone**, no authorization needed | Needs an active subscription and `now >= next_charge_ts`. Moves `amount` with `token.transfer_from` (this contract is the spender). |
| `cancel(caller, sub_id) -> Result<(), Error>` | `caller` must authorize and be the subscriber or the plan's merchant | Sets the subscription to `Cancelled`. Does not touch the token allowance. |
| `get_plan(plan_id) -> Result<Plan, Error>` | anyone | Read only. |
| `get_subscription(sub_id) -> Result<Subscription, Error>` | anyone | Read only. |
| `plan_count() -> u64` | anyone | Number of plans created. Ids run from 1 to this value. |
| `sub_count() -> u64` | anyone | Number of subscriptions created. Ids run from 1 to this value. |

Types:

```rust
pub struct Plan { merchant: Address, token: Address, amount: i128, period_secs: u64, active: bool }
pub enum SubStatus { Active, Cancelled }
pub struct Subscription { plan_id: u64, subscriber: Address, next_charge_ts: u64, status: SubStatus, charges: u32 }
```

`charges` counts successful charges after the first payment made at `subscribe`.

### Charge schedule (no back-billing)

After a successful `charge` at time `now`:

```
scheduled = next_charge_ts + period_secs
next_charge_ts = if scheduled <= now { now + period_secs } else { scheduled }
```

A late charge never collects the periods that were missed. They are skipped. Only one period is paid per call. A charge that is only slightly late keeps the original rhythm (the next one can come sooner than one full period later). Consequence, tested: after `k` successful charges at least `k * period_secs` seconds have passed since subscribing.

## Errors

| Code | Name | Raised when |
|---|---|---|
| 1 | `InvalidAmount` | `create_plan` with `amount <= 0` |
| 2 | `InvalidPeriod` | `create_plan` with a period outside 60 s to 365 days |
| 3 | `PlanNotFound` | unknown plan id (`get_plan`, `deactivate_plan`, `subscribe`) |
| 4 | `PlanInactive` | `subscribe` to a deactivated plan |
| 5 | `SubNotFound` | unknown subscription id |
| 6 | `NotActive` | `charge` or `cancel` on a cancelled subscription |
| 7 | `NotDue` | `charge` before `next_charge_ts` (including a second charge in the same period) |
| 8 | `NotAuthorized` | `cancel` by an address that is neither the subscriber nor the merchant |
| 9 | `Overflow` | an arithmetic overflow (not reachable with sane inputs) |
| 10 | `PaymentFailed` | the token transfer failed in `subscribe` or `charge`: balance too low, allowance too low or expired, or the token refused |

`PaymentFailed` does not say which reason. A client that needs to tell them apart reads `token.balance(subscriber)` and `token.allowance(subscriber, contract)`.

Why code 10 exists: the token's own error codes pass through a plain call and collide with ours (the Stellar Asset Contract's code 9 decoded as `Overflow` in the first version of the tests). Both token calls are wrapped, so clients only ever see these ten codes from a contract error.

Missing authorization is not one of these codes. It fails as a host authorization error before or inside the call.

## Events

Topic 0 is the event name as a symbol. The following topics are listed in order. The data is a map of the remaining fields keyed by field name.

| Event (topic 0) | Topics after the name | Data |
|---|---|---|
| `plan_created` | `plan_id: u64`, `merchant: Address` | `token`, `amount`, `period_secs` |
| `plan_deactivated` | `plan_id`, `merchant` | none |
| `subscribed` | `sub_id`, `plan_id`, `subscriber` | `amount` (paid now), `next_charge_ts` |
| `charged` | `sub_id`, `plan_id`, `subscriber` | `amount`, `charges` (running total), `next_charge_ts` |
| `cancelled` | `sub_id`, `plan_id`, `subscriber` | `cancelled_by` |

The token also emits its own `transfer` events inside `subscribe` and `charge`. Filter by this contract's address. Every state change emits exactly one event of this contract (a silent no-op `deactivate_plan` emits none).

## Storage

All entries are persistent, one per id. No lists, so nothing grows without bound inside one entry.

| Key | Value |
|---|---|
| `PlanCount` | `u64` counter |
| `SubCount` | `u64` counter |
| `Plan(id)` | `Plan` |
| `Sub(id)` | `Subscription` |

Ids only go up, so they are never reused, even after deactivation or cancellation.

## TTL policy

Every write extends the written entry: when its remaining life is below `TTL_THRESHOLD` (30 days, 518,400 ledgers) it is extended to `TTL_EXTEND_TO` (90 days, 1,555,200 ledgers). This assumes 5-second ledgers (measured on testnet) and stays below the testnet maximum entry TTL of 3,110,400 ledgers. `subscribe` and `charge` also extend the plan, because they depend on it.

Reads do not extend anything. A subscription that is charged each period stays alive. A cancelled or neglected entry is archived about 90 days after its last write. An archived entry has to be restored before it can be used. The two counters are written only when something is created, so on a contract that sees no new plans or subscriptions for 90 days a counter can be archived too. How the contract behaves with an archived counter has not been tested.

## Limits

- Period: `MIN_PERIOD_SECS` = 60 to `MAX_PERIOD_SECS` = 365 days. The 60 s minimum is for quick rehearsals and should be reconsidered before a mainnet release.
- Amount: `1..=i128::MAX`.
- One charge per call, and at most one per period.
- Fee and rent cost per call: **not measured yet.**

## What is not guaranteed

- **Not audited.** No independent review has happened.
- **The token is trusted.** The merchant picks the token of a plan. A token that misbehaves (blocks transfers, can freeze or claw back, charges fees) can break payments. Subscribers should only subscribe to plans with tokens they trust.
- **Charging depends on a keeper.** If nobody calls `charge`, nobody is charged. The contract does not call itself.
- **The allowance is a cap with an expiry.** When it runs out, `charge` fails with `PaymentFailed` and the subscription stays `Active` with the same due time. Nothing on-chain records failures or retries them (a grace period is a planned extension, `WAVE-ISSUE(contracts-1)`).
- **Cancel does not revoke the allowance.** The subscriber should also set the allowance to 0 at the token.
- **No refunds, no proration, no trials, no plan changes, no plan edits.** The first period is paid at `subscribe` and is not returned on cancel.
- **No upgrade path.** The deployed code cannot be changed. A bug fix means a new contract.
- **Timestamps are ledger close times**, not exact wall-clock times.
- **Time-dependent behaviour is tested with a mock clock**, and token behaviour with the built-in Stellar Asset Contract test double. Behaviour with other SEP-41 tokens and on a live network is not yet observed.
- **Authorization:** `subscribe` needs the subscriber's signature on `subscribe` and on the nested `token.transfer`. A test with exactly that authorization tree passes. Real wallet signing is not tested yet.

## Markers for planned work

`WAVE-ISSUE(contracts-1)` grace period (in `charge`), `contracts-2` trials (in `subscribe`), `contracts-3` plan changes (on `Plan`), `contracts-5` storage and TTL cost report (in `storage.rs`).
