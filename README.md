# recurring-contracts

The Soroban smart contract for Stellar Recurring: pull-based subscription billing. A merchant defines a plan (token, amount, period). A subscriber pays the first period when subscribing and grants a token allowance. After that, anyone can call a permissionless `charge` once per period.

This is **not** payment streaming. It is one discrete charge per period.

## Status

**Skeleton only.** The crate builds and has one placeholder function (`plan_count`, always 0). Plans, subscriptions, `charge` and `cancel` are not implemented yet.

**Audit status: this contract has not been audited.**

## Verify

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && stellar contract build
```

Requires Rust with the `wasm32v1-none` target and the Stellar CLI. The wasm is written to `target/wasm32v1-none/release/recurring_contracts.wasm`.

## License

MIT
