# recurring-contracts

[![ci](https://github.com/Cadence-Pay/recurring-contracts/actions/workflows/ci.yml/badge.svg)](https://github.com/Cadence-Pay/recurring-contracts/actions/workflows/ci.yml)

The Soroban smart contract for Stellar Recurring: pull-based subscription billing. A merchant defines a plan (token, amount, period). A subscriber pays the first period when subscribing and grants a token allowance. After that, anyone can call a permissionless `charge` once per period.

This is **not** payment streaming. It is one discrete charge per period.

## Status

Implemented, tested and deployed to **testnet** (not mainnet): `create_plan`, `deactivate_plan`, `subscribe`, `charge`, `cancel` and the read functions. 48 unit tests cover each function's success and failure paths and the invariants (at most one charge per period, charges never decrease, a cancelled subscription is never charged, only the two parties can cancel, ids are never reused). A full lifecycle (plan, approve, subscribe, charge by a third party, cancel) was run on the live testnet, twice, and behaved as the tests predict. See [docs/CONTRACT.md](docs/CONTRACT.md) for functions, errors, events, storage, TTL policy, measured fees, limits and what is not guaranteed.

Not done: a mainnet deployment, tests with tokens other than the Stellar Asset Contract, and an independent audit.

**Audit status: this contract has not been audited.**

## On testnet

The contract id, the wasm hash (the built file matches the deployed code), the test accounts and every transaction of the live runs are in [docs/TESTNET.md](docs/TESTNET.md) and [docs/DEMO.md](docs/DEMO.md). Testnet is wiped on its reset dates (the next one is 2026-12-16); after that the contract has to be deployed again and gets a new id.

## Verify

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && stellar contract build
```

Requires Rust with the `wasm32v1-none` target and the Stellar CLI. The wasm is written to `target/wasm32v1-none/release/recurring_contracts.wasm`.

## Documents

| Document                                               | What it holds                                                                                |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------- |
| [docs/CONTRACT.md](docs/CONTRACT.md)                   | the reference: functions, errors, events, storage, TTL, fees, limits                         |
| [docs/TESTNET.md](docs/TESTNET.md)                     | the testnet deployment and the first live run                                                |
| [docs/DEMO.md](docs/DEMO.md)                           | what was run end to end, with dates and transaction hashes, and what was not tested          |
| [docs/RELEASE.md](docs/RELEASE.md)                     | the release runbook for the scripts in [scripts/](scripts/), run on testnet only             |
| [docs/MAINNET_READINESS.md](docs/MAINNET_READINESS.md) | what is missing before a mainnet release (on hold: the project stays on testnet)             |
| [docs/RESEARCH.md](docs/RESEARCH.md)                   | what was checked in the Stellar docs and on the network while building, with dates and links |

## Stellar Recurring repositories

| Repository                                                                | What it is                                                                   |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| [recurring-contracts](https://github.com/Cadence-Pay/recurring-contracts) | this contract: plans, subscriptions, `charge`, `cancel`                      |
| [recurring-keeper](https://github.com/Cadence-Pay/recurring-keeper)       | service that charges due subscriptions, sends webhooks and serves a read API |
| [recurring-dashboard](https://github.com/Cadence-Pay/recurring-dashboard) | Next.js dashboard for merchants                                              |
| [recurring-checkout](https://github.com/Cadence-Pay/recurring-checkout)   | React subscribe button for websites                                          |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Report security problems privately as described in [SECURITY.md](SECURITY.md), never in a public issue.

## License

MIT
