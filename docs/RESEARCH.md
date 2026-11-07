# RESEARCH

> This is the research log kept while the project was built in one repository, before it was split into recurring-contracts, recurring-keeper, recurring-dashboard and recurring-checkout. Mentions of the playbook, `STATUS.md` and task numbers refer to planning files of that build that are not part of this repository. The findings themselves are dated and linked to their sources.

Checked: 2026-11-05. Sources are linked per item. "Verified" means I ran it or read the source text on that date. "Unverified" means I did not confirm it and the task that needs it must check first.

## Environment (task 0.1)

| Tool | Result |
|---|---|
| git | 2.49.0.windows.1 |
| node / npm | v24.18.0 / 11.16.0 |
| cargo / rustc | 1.99.0 |
| stellar CLI | 30.0.1 (stellar-xdr 30.0.0) |
| rust targets | `wasm32-unknown-unknown`, `wasm32v1-none`, `x86_64-pc-windows-msvc` |
| corepack | 0.35.0 |
| pnpm | **11.28.9**, activated through corepack |

pnpm notes:
- `corepack enable` failed with EPERM because it writes shims into `C:\Program Files\nodejs`. With the user's approval, the shims were installed with `corepack enable --install-directory %USERPROFILE%\.corepack-bin`, and that folder was added to the user PATH. Open a new terminal to pick it up.
- The newest pnpm (12.16.1) hung for over 10 minutes in `corepack prepare` while "downloading the pnpm binary for win32-x64". The tarball itself downloads in about 6 seconds, so the cause is unknown. pnpm 11.28.9 activated in 120 seconds, so that is the pinned version. Revisit pnpm 12 later.
- `pnpm --version` printed a warning that a `workspaces` field in some `package.json` is unsupported. It came from a parent folder, not from this repo. Ignore it, but do not put a `workspaces` field in any package here.

## 1. SEP-41 token interface

Source: [SEP-0041 v0.5.2](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md) (Draft, updated 2026-08-24). Verified from the raw text.

```rust
fn allowance(env: Env, from: Address, spender: Address) -> i128;
fn approve(env: Env, from: Address, spender: Address, amount: i128, live_until_ledger: u32);
fn balance(env: Env, id: Address) -> i128;
fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128);
fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128);
```

Rules that matter for this project:
- `approve` **overwrites** the existing allowance. It does not add to it. Setting amount to 0 revokes.
- `live_until_ledger` cannot be lower than the current ledger unless amount is 0. An expired allowance counts as 0.
- An allowance is a spending limit, not a reservation. It may exceed the balance.
- `transfer_from` needs `spender.require_auth()`. Here the spender is the recurring contract, which is the invoker, so no signature is needed. It lowers the allowance and does not change the expiry.
- `transfer_from` **fails** if the allowance or the balance is less than `amount`. The spec gives no error codes. **Observed in the 1.4 tests (soroban-sdk 29.0.1 test double of the SAC):** a missing, too small or expired allowance fails with contract error code 9, which collides with our own code 9. The contract therefore catches the token failure and returns `PaymentFailed` (code 10) instead; the call still reverts (decision D3, D15).
- `transfer` takes a `MuxedAddress` as `to`, while `transfer_from` takes a plain `Address`. `subscribe` moves the first payment with `transfer` and must pass an `Address` converted to `MuxedAddress`. Check the conversion in the SDK docs while writing 1.3.
- SEP-41 has a warning about changing an allowance: replacing it can let the spender use the old one first. For us the spender is our own contract, which only spends on `charge`, so the risk is low. The checkout widget should still show the cap and expiry.

## 2. Can approve and subscribe share one transaction?

Sources: [Operations and transactions](https://developers.stellar.org/docs/learn/fundamentals/transactions/operations-and-transactions), [Authorization](https://developers.stellar.org/docs/learn/fundamentals/contract-development/authorization). Verified from the doc text.

- Docs quote: "Smart contract transactions (those with `InvokeHostFunctionOp`, `ExtendFootprintTTLOp`, or `RestoreFootprintOp` operations) can only have one operation per transaction."
- So `token.approve(...)` and `recurring.subscribe(...)` cannot be two operations in one transaction.
- The only way to do both atomically is **sub-invocation**. The Authorization page says a contract can call `require_auth` for an address and then call a token function authorized for the same address, with no special code. In that design `subscribe` would itself call `token.approve(subscriber, this_contract, cap, expiry)`.
- That design needs extra arguments on `subscribe` (cap, expiry), which differs from the §4 signature. It is **unverified** in practice: it has not been tested in a Soroban test with a real token.

**Decision for the checkout flow (proposal, confirmed at Gate 1):** two transactions. Transaction 1 is `token.approve(subscriber, recurring_contract, cap, live_until_ledger)`. Transaction 2 is `recurring.subscribe(subscriber, plan_id)`. The widget must handle the case where the user signs the first and rejects or fails the second (allowance set, no subscription). Order matters: approve first, because `subscribe` does not need an allowance (D1), but `charge` does.
The sub-invocation variant is raised as an option at Gate 1. It would trade a signature change for one signature prompt instead of two.

Related limit: the longest an allowance can last is bounded by the network's maximum entry TTL (see item 7, 3,110,400 ledgers, about 180 days at 5 s per ledger). **Verified on testnet 2026-11-05:** the native XLM Stellar Asset Contract rejected `approve` with `live_until_ledger` beyond the latest ledger plus 3,110,400 and accepted values up to it (see `docs/TESTNET.md`). So the widget has to tell subscribers they must re-approve at least about every 6 months, and the docs must say so.

## 3. soroban-sdk

Sources: [crates.io](https://crates.io/crates/soroban-sdk), [docs.rs 29.0.1](https://docs.rs/soroban-sdk/29.0.1/soroban_sdk/). Verified.

- Newest **stable** is **29.0.1** (published 2026-10-12 per docs.rs). A `30.0.0-rc.1` exists (2026-10-21). The installed CLI is 30.0.1, one major above the stable SDK. Compatibility of an SDK 29 build with the testnet protocol is **unverified**. The first build and deploy (1.1, 1.7) will show it. Use 29.0.1 and record the result.
- Macros present in the docs: `contracttype`, `contracterror`, `contractimpl`, `contractevent`. Exact attribute usage for `contracttype` and `contracterror` will be taken from the docs pages while writing 1.1.
- Events: `#[contractevent]` on a struct. Fields marked `#[topic]` become topics, the rest become the data map, and the first topic is the struct name in snake case (so `struct Charged` publishes topic `charged`). A custom fixed topic list is possible with `#[contractevent(topics = [...])]`. Publishing is done from the struct (method name to confirm in the docs page in 1.2).
- TTL: `env.storage().persistent().extend_ttl(&key, threshold, extend_to)`. It extends only if the current TTL is below `threshold`, and then sets it to `extend_to`. There is also an instance-storage version, and a separate `extend_ttl_with_limits`.
- Tests: with the `testutils` feature, `env.mock_all_auths()`, `env.mock_auths(&[...])`, `env.auths()`, `env.ledger().set_timestamp(u64)`, `env.ledger().set_sequence_number(u32)`, `env.register(contract, ctor_args)` and `env.register_stellar_asset_contract_v2(admin)` all exist. Use `mock_auths` rather than `mock_all_auths` in tests that check who is allowed to act (cancel by a stranger, non-merchant deactivate). `mock_all_auths` would hide auth bugs.
- `env.ledger().timestamp()` is the ledger time source. The `Ledger` trait in the test utilities has `set_timestamp`.

## 4. Stellar RPC

Sources: [getEvents](https://developers.stellar.org/docs/data/apis/rpc/api-reference/methods/getEvents), [simulateTransaction](https://developers.stellar.org/docs/data/apis/rpc/api-reference/methods/simulateTransaction), [Providers](https://developers.stellar.org/docs/data/apis/rpc/providers). Retention verified live on testnet; the rest verified from docs text.

- Methods: `simulateTransaction`, `sendTransaction`, `getTransaction`, `getEvents`, `getLedgerEntries` (used to read contract state), `getHealth`, `getLatestLedger`. Their exact request shapes come from `@stellar/stellar-sdk`'s `rpc.Server` in Phase 2 (check the installed source).
- **getEvents retention:** the default history window is 120,960 ledgers, about 7 days. A live `getHealth` call on `https://soroban-testnet.stellar.org` returned `ledgerRetentionWindow: 120960`, `oldestLedger: 5449359`, `latestLedger: 5570318`. The two close times differ by 604,795 s, which is 7 days, so a ledger closes about every 5 s.
- `getEvents`: `limit` 1 to 10000, default 100. It errors if `startLedger` is older than the oldest stored ledger. So events can only back-fill about 7 days. This supports decision D4 (events are an optimization; state reads are the source of truth). Providers may use different windows. Check each one.
- Rate limits for the SDF testnet RPC are **not documented** on the pages I read. Unverified. The keeper must back off on HTTP 429 and be configurable.
- Reading state: `plan_count` / `sub_count` / `get_*` are read-only contract calls. Run them with `simulateTransaction`, no signing or fee.

## 5. TypeScript bindings

Sources: [stellar-cli reference](https://developers.stellar.org/docs/tools/cli/stellar-cli), [js-stellar-sdk README](https://github.com/stellar/js-stellar-sdk#cli). Verified from docs and from `stellar --help` text.

- **`stellar contract bindings typescript` is deprecated** in CLI 30.0.1. Its help says "Deprecated, use the JavaScript Stellar SDK instead".
- Replacement:
  ```bash
  npx @stellar/stellar-sdk generate --wasm ./target/wasm32v1-none/release/recurring_contracts.wasm \
    --output-dir ./src/contract --contract-name recurring-contracts
  # or from a deployed contract:
  npx @stellar/stellar-sdk generate --contract-id C... --network testnet --output-dir ./src/contract
  ```
  Mainnet needs `--rpc-url` (no default). Flags: `--wasm`, `--wasm-hash`, `--contract-id`, `--network`, `--rpc-url`, `--output-dir`, `--contract-name`, `--overwrite`, `--allow-http`, `--timeout`, `--headers`.
- Output layout (per README): `src/index.ts`, `src/client.ts`, `src/types.ts`, `package.json`, `tsconfig.json`, `README.md`, `.gitignore`. Phase 2.1 must trim the generated `package.json` etc. so the bindings live inside the keeper's `src/contract/` and still lint and typecheck.
- The wasm path above is an assumption (`wasm32v1-none` target, crate name `recurring_contracts`). Confirm after the first build.
- This changes the playbook text ("research item 5") but not the plan. Phase 2.1 just uses the new command. No decision in `STATUS.md` changes.

## 6. Wallet integration

Sources: [Stellar Wallets Kit README](https://github.com/Creit-Tech/Stellar-Wallets-Kit), npm registry. Verified for names and versions; the API shape is from the README only.

- npm package **`@creit.tech/stellar-wallets-kit`**, latest **2.8.0**. The README installs from JSR as `@creit-tech/stellar-wallets-kit` (`npx jsr add`). The names differ (`.` vs `-`), so use the npm name in `package.json` and check it installs.
- Exports (from npm `exports`): `.`, `./sdk`, `./modules/utils`, `./modules/freighter`, `./modules/xbull`, `./modules/albedo`, `./modules/lobstr`, `./modules/wallet-connect`, and others. No peer dependencies listed.
- API in 2.x is a static class:
  ```ts
  import { StellarWalletsKit } from '@creit.tech/stellar-wallets-kit/sdk';
  import { defaultModules } from '@creit.tech/stellar-wallets-kit/modules/utils';
  StellarWalletsKit.init({ modules: defaultModules() });
  const { address } = await StellarWalletsKit.getAddress();
  const { signedTxXdr } = await StellarWalletsKit.signTransaction(xdr, { networkPassphrase, address });
  ```
  (The README writes the scope as `@creit-tech`; I used the npm name. Confirm the import path after install.)
- Signing a transaction that has sub-invocation auth entries, and signing auth entries on their own, are **not covered** by the README. Check before 3.3 and 4.2.
- It touches `window`, so it must only load on the client in Next.js (dynamic import).
- The README's wallet list shows "Varial, formerly Freighter". Treat wallet names as unverified.

## 7. Networks

Sources: [Networks](https://developers.stellar.org/docs/networks), [RPC providers](https://developers.stellar.org/docs/data/apis/rpc/providers), [Circle USDC addresses](https://developers.circle.com/stablecoins/usdc-contract-addresses), live `stellar network settings` on testnet. Verified unless stated.

| | Testnet | Mainnet |
|---|---|---|
| Passphrase | `Test SDF Network ; September 2015` | `Public Global Stellar Network ; September 2015` |
| RPC | SDF: `https://soroban-testnet.stellar.org` | SDF runs none. Third-party only (below) |
| Friendbot | `https://friendbot.stellar.org`, 10,000 XLM | none |
| Resets | 2 to 4 per year; **next scheduled 2026-12-16 17:00 UTC** | never |

Public mainnet RPC options (from the providers page; rate limits and `getEvents` window are not listed there, so check each provider's terms before Phase 7):
- `https://mainnet.sorobanrpc.com` (sorobanrpc.com)
- `https://soroban-rpc.mainnet.stellar.gateway.fm` (Gateway)
- `https://stellar-soroban-public.nodies.app` (Nodies)
- `https://stellar.api.onfinality.io/public` (OnFinality)
- Paid or keyed: Blockdaemon, Validation Cloud, QuickNode, Ankr, Alchemy and others.

**The testnet reset on 2026-12-16 will wipe the deployed contract and test accounts.** Plan Phase 1.7 and the demo knowing this. `docs/TESTNET.md` must say when things were deployed, and a redeploy script or steps should exist.

**Test token on testnet:** a Stellar Asset Contract (SAC) wrapping a classic asset. Create the classic asset with an issuer account, then `stellar contract asset deploy --asset CODE:ISSUER --source-account ... --network testnet`. Circle's testnet USDC classic asset is `USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5`. Its SAC id, derived locally with `stellar contract id asset` (no secret involved), is `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA`. Getting test USDC needs a trustline and a faucet, so a self-issued asset is simpler for rehearsal.

**Mainnet USDC:** Circle documents USDC on Stellar as the classic asset `USDC-GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN`. Circle's page does not give a contract id. The SAC id is derived by the CLI: `CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75`. Before Phase 7, re-derive it and compare it with an independent source such as stellar.expert.
Note: Circle can freeze or claw back USDC (issuer controls). That is a risk to mention in the mainnet docs.

**CLI usage (from the CLI reference):**
```bash
stellar keys generate deployer --network testnet --fund
stellar contract build
stellar contract deploy --wasm <file.wasm> --source-account deployer --network testnet --alias recurring
stellar contract invoke --id recurring --source-account merchant --network testnet --cost -- create_plan --merchant merchant --token <TOKEN> --amount 10000000 --period_secs 60
```
Notes: `--cost` prints the execution cost to stderr. `--send=no` simulates only. `--build-only` prints the unsigned XDR. For mainnet, `--rpc-url` and `--network-passphrase` are passed explicitly or set with `stellar network add`. `contract invoke` names the contract with `--contract-id` (alias form `--id`). I did not run any of these commands (no contract exists yet). Key and flag names above are from the reference only.

**Fee and resource budget (testnet, live on 2026-11-05):**
- Per-transaction limits: 400,000,000 instructions, 200 disk-read entries, 200 write entries, 132,096 write bytes, 132,096 tx bytes, 16,384 bytes of events.
- Ledger: 2,000 contract transactions.
- State archival: minimum persistent TTL 120,960 ledgers (about 7 days), **maximum entry TTL 3,110,400 ledgers (about 180 days)**, persistent rent rate denominator 1215.
- Fee rates: 781 per disk read entry, 1,250 per write entry, 203 per KB of transaction size, 2,500 per KB of events (stroop-scale units as reported by the CLI; I did not convert these to XLM).
- The contract's actual per-call cost and rent are **not measured yet**. Task 1.7 must record `--cost` output for `subscribe` and `charge`, and Phase 7.1 turns it into a mainnet budget. Mainnet limits are set by validator vote and can differ: re-run `stellar network settings` against mainnet in 7.1.

## 8. Next.js

Source: [Next.js installation docs](https://nextjs.org/docs/app/getting-started/installation). Version verified on npm.

- Current stable: **next 16.4.5**. Minimum Node.js 20.9. Our Node is 24.18.0.
- App Router (`app/` folder, server components by default, `"use client"` for wallet code). `create-next-app` exists, but Phase 0.2 can write the skeleton by hand to avoid extra dependencies.

## Other version checks (npm latest on 2026-11-05)

| Package | Version | Note |
|---|---|---|
| `@stellar/stellar-sdk` | 17.4.0 | Verify the `rpc.Server` and `contract.Client` names in the installed source |
| `typescript` | 7.0.2 | A new major. Check that `typescript-eslint` and `vitest` work with it in 0.2, else pin the previous major and note why |
| `vitest` | 5.1.0 | |
| `zod` | 4.6.7 | |
| `tsup` | 9.0.0 | |
| `fastify` | 5.12.8 | |
| `hono` | 4.13.16 | |

## Keeper decisions needed from research

- **HTTP framework: `fastify`.** Both are on the allowed list. Hono needs an extra adapter package (`@hono/node-server`) to run on Node, which is not in the playbook list. Fastify runs on Node directly. Validate request input with `zod` inside the handlers (a Fastify zod plugin would also be an unlisted dependency).
- **SQLite: `node:sqlite`.** `require('node:sqlite')` loads on Node 24.18.0 and exposes `DatabaseSync` (SQLite 3.53.1). The Node docs' stability label for it was **not checked**. Check [the Node sqlite docs](https://nodejs.org/api/sqlite.html) in task 2.3 and fall back to `better-sqlite3` only if it is not stable.

## Changes to decisions in STATUS.md

None of D1 to D6 changes. Findings that touch them:
- D1 holds. Item 2 shows a two-transaction checkout flow is the safe default.
- D4 is supported. Event retention is only about 7 days.
- New risk for D6 and Phase 7: allowance expiry is capped at roughly 180 days.
