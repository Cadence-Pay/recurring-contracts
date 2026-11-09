# Mainnet readiness review

> **On hold (user decision of 2026-11-06): the project stays on testnet.** This review is kept for a possible later mainnet release. Nothing in it is needed for testnet.

Written 2026-11-06. **No network action was taken to write this.** Nothing has been deployed to mainnet and no mainnet key has been created, seen or used. Every figure below that comes from the network was measured on **testnet** and must be measured again on mainnet before relying on it; this is marked where it matters.

It is written for the maintainers, who decide whether and when to release.

## 1. Audit status

**The contract has not been audited.** Nobody outside this project has reviewed it. The keeper, dashboard and checkout widget have not been audited either. What exists is the author's own testing: 48 contract unit tests (including property-style invariant tests), a full lifecycle run on testnet, and the tests in the other three packages (see each README).

**Recommendation: get an independent review or audit of the contract before it handles real money.** The contract is small (about 13 KB of wasm, nine functions), which keeps a review short, but it moves people's tokens and has no upgrade path (section 5). The decision to proceed without one is yours; if you do, say so plainly in the READMEs (they already say "not audited").

## 2. Known limits and risks

These are limits of the design as built. They are also in the package READMEs and [CONTRACT.md](CONTRACT.md).

**Allowance model**
- A subscriber approves a capped, expiring token allowance for the contract. The network limits how far in the future an approval can end (about 180 days, `max_entry_ttl` 3,110,400 ledgers on testnet; **check mainnet's value**). So a subscriber must re-approve at least about every six months, and a plan with a long period covers fewer charges per approval. The widget says how many; a period longer than any approval can last cannot be subscribed to with the widget.
- Cancelling a subscription does not remove the approval. The subscriber should also remove it in their wallet.
- The first period is paid at subscribe and is not refunded on cancel.

**The token is trusted.** The merchant chooses the plan's token. A token that can freeze accounts or claw back funds (for example USDC, where the issuer holds those powers), charges fees, or blocks transfers can break payments or take funds. Subscribers must only subscribe to plans with tokens they trust. Mainnet USDC on Stellar is a classic asset wrapped as a contract; its contract id is derived from the issuer, and **must be re-derived and compared with an independent source before use** (see `RESEARCH.md`; never copy an id from this document).

**Keeper liveness.** If no keeper runs, nobody is charged. Charging is permissionless, so anyone can run one, but this project runs none for you. A subscription that is not charged on time keeps its due time and is charged later, once, without back-billing (missed periods are skipped).

**RPC limits.** Public RPC servers rate-limit and keep about seven days of history. On testnet the public RPC timed out twice during the end-to-end run (the keeper's retry design handled it). Mainnet has no public RPC run by the Stellar Development Foundation; a provider with a key is needed, and its terms and `getEvents` retention must be checked.

**No upgrade path.** The deployed code cannot be changed. A bug fix means deploying a new contract and moving people over by hand. This was a deliberate choice (decision D6) for a simple trust model, and it is the main reason to want an audit first.

**Storage and expiry.** Entries are persistent and are extended on every write (to 90 days when below 30). An entry that is not written for about 90 days is archived and has to be restored before it can be used. A cancelled or quiet subscription therefore eventually becomes inactive state; counters could be archived on a contract that sees no new plans or subscriptions for 90 days. How the contract behaves with an archived counter has **not been tested**.

**Other limits to be aware of**
- The minimum billing period is 60 seconds. That value exists for quick rehearsals; **reconsider it before mainnet** (for example a day, or an hour).
- Charges can fail for reasons the contract does not tell apart (`PaymentFailed`: low balance, low or expired allowance, or the token refusing).
- A failed charge is not recorded on-chain; failure tracking is off-chain in the keeper. There is no on-chain grace period.
- Webhooks have only been tested against fake receivers.
- The dashboard has been used by a person with a real wallet in a browser on testnet (create and deactivate a plan, 2026-11-09, see `DEMO.md`). Cancelling through the dashboard and the checkout widget have not been tried by a person.
- Several keepers can run, but they will race for the same charge and the losers waste a fee.
- No personal data is on-chain. The keeper stores addresses, ids, amounts, timestamps and status only.

## 3. Fee and rent budget

Measured on **testnet** on 2026-11-05, charged after refunds (from [CONTRACT.md](CONTRACT.md)). One XLM is 10,000,000 stroops.

| Action | Stroops | About (XLM) | Who pays | Note |
|---|---|---|---|---|
| Deploy (13,246-byte wasm) | 14,858 | 0.0015 | deployer | once |
| `create_plan` | 397,000 to 524,000 | 0.04 to 0.05 | merchant | mostly rent for 90 days |
| Token `approve` | 6,226 | 0.0006 | subscriber | |
| `subscribe` | 508,738 | 0.051 | subscriber | mostly rent, plus the first payment |
| `charge` | 10,873 | 0.0011 | whoever calls it (the keeper) | the recurring cost |
| `cancel` | 9,273 | 0.0009 | caller | |

How to read it for a first release: a keeper that charges 100 subscribers once a month spends about 100 × 0.0011 = 0.11 XLM a month in transaction fees; creating one plan costs about 0.05 XLM. The keeper account only pays fees. It never holds subscribers' money.

**Why these must be re-measured on mainnet:** fee rates and the limits that set them are network settings voted by validators, and they differ between networks and over time. `stellar network settings` shows them. The first step of the release kit measures the real cost of each action on the target network with `--cost` and compares it with this table. Also note that the price of XLM changes what these amounts are worth.

**Spend limits the keeper provides:** `MAX_FEE_STROOPS` refuses to submit any single charge whose fee is above the cap, and fails closed if the fee is unknown. There is no cumulative cap: the keeper account's balance is the hard bound on spending, so **fund it with only the budget you intend to spend**, and alert when it runs low (section 6).

## 4. The exact build

| Item | Value |
|---|---|
| Source | `recurring-contracts` at commit **`93f9175`** (2026-11-05), the last commit that changed contract source; later commits changed tests and documents only |
| Wasm hash (SHA-256 of the optimized wasm) | **`dde9e953b6c94497f390ae99318a77f6f3937dacd260434648747c8183e68e3c`** |
| Wasm size | 13,246 bytes |
| Built with | `stellar contract build`, rustc 1.99.0, stellar CLI 30.0.1, soroban-sdk 29.0.1, target `wasm32v1-none`, release profile from `Cargo.toml` (`opt-level = "z"`, LTO) |
| Reproduced | rebuilt on 2026-11-06 on the same machine; same hash |
| Deployed on testnet | the same hash, verified by fetching the code back from the network (`TESTNET.md`) |

**Not yet shown:** that a build on a different machine gives the same hash. The release kit rebuilds immediately before deploying and refuses to deploy if the hash differs from the one you approve; after deploying it fetches the code back from the network and refuses to continue if the hash differs.

## 5. If a bug is found: what can and cannot be done

Because there is no upgrade path, a bug in the deployed contract cannot be patched. What exists:

- **Merchants** can deactivate plans (no new subscriptions) and cancel any subscription on their plans.
- **Subscribers** can cancel their subscriptions. **The one hard stop for a subscriber is to set the token allowance for the contract to 0 in their wallet**, which makes every future charge fail. Cancelling alone also stops charges, but only through the same contract.
- **The keeper** can be stopped. But `charge` is permissionless: stopping this keeper does not stop others, and it does not stop a bug in `charge` from being used by anyone. Only cancelling and removing allowances do.
- **A fix** is a new contract, deployed from the release kit, with plans re-created on it and subscribers asked to subscribe again.

Write down, before launch, who you would tell and how (merchants first), and keep `SECURITY.md` current with a working private channel.

## 6. Monitoring and alerting for the keeper

What exists today: `GET /health` (status, network, contract id, read-only flag and `lastSyncAt`), structured JSON logs, and a health check script (`recurring-keeper/scripts/check-health.ts`) that exits non-zero when the keeper is down, not syncing, or read-only when it should be charging, so it can be run from a scheduler. There is no metrics endpoint (that is a contributor issue, `keeper-4`).

Alert on:

| Condition | How to see it |
|---|---|
| The keeper process is down | the health check fails to connect |
| It stopped syncing | `lastSyncAt` older than five poll intervals |
| It is charging nothing it should | `readOnly` is true on a keeper that should charge |
| Charges keep failing | log lines `charge failed, giving up for now`, and `charge.failed` webhooks |
| RPC trouble | repeated `keeper pass failed` log lines |
| The keeper account is nearly empty | check its balance on a schedule (CLI: `stellar contract invoke` on the native token `balance`, or any explorer) |

A pager or email integration is not provided; wire the health check and a log watcher to whatever you use.

## 7. What must be true before you say go

- [ ] You have decided about an independent audit (section 1).
- [ ] You have reviewed section 2, and decided about the 60 second minimum period.
- [ ] You have chosen the token and checked its contract id against an independent source.
- [ ] You have a mainnet RPC provider and have read its limits and terms.
- [ ] You have created and funded the mainnet identities yourself (the owner's action list is kept outside the package repositories) and you have set a fee budget.
- [ ] You have read `RELEASE.md` and are comfortable with each step.
