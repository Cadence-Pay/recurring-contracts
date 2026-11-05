# recurring-contracts

The Soroban smart contract for Stellar Recurring: pull-based subscription billing. A merchant defines a plan (token, amount, period). A subscriber pays the first period when subscribing and grants a token allowance. After that, anyone can call a permissionless `charge` once per period.

This is **not** payment streaming. It is one discrete charge per period.

## Status

Implemented and covered by 48 unit tests: `create_plan`, `deactivate_plan`, `subscribe`, `charge`, `cancel` and the read functions. See [docs/CONTRACT.md](docs/CONTRACT.md) for functions, errors, events, storage, TTL policy, limits and what is not guaranteed.

Not done yet: testnet deployment, fee and rent measurements, and any test against a live network or a real wallet.

**Audit status: this contract has not been audited.**

## Verify

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && stellar contract build
```

Requires Rust with the `wasm32v1-none` target and the Stellar CLI. The wasm is written to `target/wasm32v1-none/release/recurring_contracts.wasm`.

## License

MIT
