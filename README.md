# recurring-contracts

The Soroban smart contract for Stellar Recurring: pull-based subscription billing. A merchant defines a plan (token, amount, period). A subscriber pays the first period when subscribing and grants a token allowance. After that, anyone can call a permissionless `charge` once per period.

This is **not** payment streaming. It is one discrete charge per period.

## Status

Implemented, tested and deployed to **testnet** (not mainnet): `create_plan`, `deactivate_plan`, `subscribe`, `charge`, `cancel` and the read functions. 48 unit tests cover each function's success and failure paths and the invariants (at most one charge per period, charges never decrease, a cancelled subscription is never charged, only the two parties can cancel, ids are never reused). A full lifecycle (plan, approve, subscribe, charge by a third party, cancel) was run on the live testnet, twice, and behaved as the tests predict. See [docs/CONTRACT.md](docs/CONTRACT.md) for functions, errors, events, storage, TTL policy, measured fees, limits and what is not guaranteed.

Not done: a mainnet deployment, tests with tokens other than the Stellar Asset Contract, and an independent audit.

**Audit status: this contract has not been audited.**

## Verify

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && stellar contract build
```

Requires Rust with the `wasm32v1-none` target and the Stellar CLI. The wasm is written to `target/wasm32v1-none/release/recurring_contracts.wasm`.

## License

MIT
