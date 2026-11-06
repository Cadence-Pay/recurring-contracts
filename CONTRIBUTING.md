# Contributing to recurring-contracts

Thank you for helping. This is the Soroban smart contract of Stellar Recurring: pull-based subscription billing on Stellar. One discrete charge per period; it is **not** payment streaming.

The contract holds the rules for moving people's tokens. Treat any bug as potentially affecting funds.

## Set up

You need Rust (stable) with the `wasm32v1-none` target, and the [Stellar CLI](https://developers.stellar.org/docs/tools/cli).

```bash
rustup target add wasm32v1-none
cd recurring-contracts
cargo test
```

## Before you open a pull request

Run the full check. A pull request is ready when this passes:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && stellar contract build
```

Rules for this package:

- Contract code is `#![no_std]`. No `unwrap`, `expect` or `panic!`. Failures return a variant of the `Error` enum.
- Every persistent write extends the entry's TTL. Every state change emits an event. No unbounded lists.
- Every public function has a test for success and at least one for failure. Use `mock_auths`, not `mock_all_auths`, in tests that are about who is allowed to act.
- Changing a public function, an error code or an event changes the contract's interface. Say so in the pull request and update `docs/CONTRACT.md` in the same change.

Where things are: `src/` (contract), `docs/CONTRACT.md` (reference), tests in `src/test.rs`.

General rules:

- **Testnet only.** Never put a mainnet key, a seed phrase or any secret in a file, a test, a log, a commit or an issue. Use the Stellar CLI key store.
- **Describe only what exists.** Documentation says what is implemented and tested. No claims about features that are not there.
- **Keep to the issue.** Do what the issue asks and no more. A small, focused change is reviewed faster than a large one.
- **Test it.** New behaviour comes with tests for success and for failure. Say in the pull request what you ran.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org). Examples:

```
feat(contracts): add <what>
fix(contracts): handle <case>
docs(contracts): explain <thing>
test(contracts): cover <case>
```

## Claiming an issue

1. Pick an issue that is not assigned. Issues drafted for contributors name a marker such as `WAVE-ISSUE(...)`; search the code for it to find where the work plugs in.
2. Comment that you would like to take it, and wait to be assigned. One person per issue at a time.
3. Ask questions in the issue if the scope is unclear. The scope written there is the limit of the change.
4. Open a pull request that mentions the issue. If you stop working on it, say so, so someone else can take it.

## Reporting a security problem

Do not open a public issue. See [SECURITY.md](SECURITY.md).

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
