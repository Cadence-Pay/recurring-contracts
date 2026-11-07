# Release runbook

> **Testnet only, for now (user decision of 2026-11-06).** Run every step on testnet and leave the mainnet guard flags (`RELEASE_ALLOW_MAINNET`, `E2E_ALLOW_MAINNET`) unset. Mainnet is on hold; the parts below about mainnet are kept for a possible later decision.

How to release Stellar Recurring to a network, step by step, using the release kit. The whole sequence has been run on **testnet** with exactly these scripts (see `DEMO.md`). It has **never been run on mainnet**.

**Mainnet moves real value and cannot be undone.** Read [MAINNET_READINESS.md](MAINNET_READINESS.md) first. The kit refuses mainnet unless the owner sets `RELEASE_ALLOW_MAINNET=yes` (and `E2E_ALLOW_MAINNET=yes` for the subscribe step) on the command being run. The agent never sets them unless you have said go, and never creates, funds or looks at a mainnet key.

## What the kit is

| Piece | Where | Does |
|---|---|---|
| `release.mjs` | `recurring-contracts/scripts/` | build, deploy with a hash check, create a plan, cancel, read state, revoke an approval |
| `e2e-subscribe.test.ts` | `recurring-checkout/scripts/` | subscribe through the widget's real flow (balance check, approve, subscribe) |
| The keeper | `recurring-keeper/` | the charge loop, with a fee cap |
| `wait-for-charge.ts`, `check-health.ts` | `recurring-keeper/scripts/` | wait for a charge to succeed; health check for a scheduler |

What it refuses, by design: a secret key or seed phrase where an identity **name** belongs (it is not used and not echoed); a network name whose passphrase does not match; mainnet without the guard; a deploy whose built hash is not the one you approved; and continuing after a deploy if the code fetched back from the network differs from the build.

Keys never pass through the kit. Every step names an identity in the Stellar CLI key store, and the CLI does the signing.

## What you provide

- Funded identities in the CLI key store, by **name**: a deployer, a merchant, a subscriber (an account you own) and a keeper.
- An RPC provider URL (mainnet has no public RPC run by the Stellar Development Foundation) and its terms.
- The token contract id, checked against an independent source.
- A fee budget, and how long the keeper should keep running.

Set these for every command (they are never built into the code):

```bash
export STELLAR_NETWORK=testnet            # or mainnet
export STELLAR_RPC_URL=https://...        # your RPC server
export STELLAR_NETWORK_PASSPHRASE="..."   # must match the network
```

Testnet values: `https://soroban-testnet.stellar.org` and `Test SDF Network ; September 2015`. Mainnet passphrase: `Public Global Stellar Network ; September 2015`.

## Stop rules

Stop and report, without trying to carry on, if any of these happens:

- a hash differs (built hash against the approved one, or on-chain code against the build);
- a step costs more than the budget you gave, or more than about twice the figures in `MAINNET_READINESS.md` section 3;
- any unexpected error, or a result that is not what the step says it should be.

Nothing after the failed step is run.

## The steps

### 0. Preflight (read only)

```bash
stellar network settings --rpc-url "$STELLAR_RPC_URL" --network-passphrase "$STELLAR_NETWORK_PASSPHRASE"
```

Compare fee rates and `max_entry_ttl` with `MAINNET_READINESS.md` (the widget's approval length assumes at least 3,000,000 ledgers). Derive the token contract id yourself and compare it with an independent source:

```bash
stellar contract id asset --asset USDC:<issuer> --rpc-url "$STELLAR_RPC_URL" --network-passphrase "$STELLAR_NETWORK_PASSPHRASE"
```

Check that the deployer, merchant, subscriber and keeper accounts have the funds you intended, and no more than the budget.

### 1. Build and approve the hash

```bash
cd recurring-contracts
node scripts/release.mjs build
```

It prints `WASM_SHA256=...`. The approved hash is the one in `MAINNET_READINESS.md` section 4 (`dde9e953b6c94497f390ae99318a77f6f3937dacd260434648747c8183e68e3c` for commit `93f9175`). If the source changed, review the change, build, and write the new approved hash there before going on.

### 2. Deploy, verify the hash

```bash
RELEASE_ALLOW_MAINNET=yes node scripts/release.mjs deploy --source <deployer> \
  --expect-hash <approved hash> --alias <name> --out release.json
```

(`RELEASE_ALLOW_MAINNET=yes` only for mainnet.) It rebuilds, refuses if the hash is not the approved one, deploys, fetches the code back from the network and refuses if it differs. It prints `CONTRACT_ID=...` and writes `release.json` (public data only: network, contract id, transaction hash, hash, time). Record the contract id and transaction hash in `docs/MAINNET.md`. The fee in that file was wrong before the parser was fixed in commit `5841411`; the printed per-step fees of later steps are right.

### 3. Create the first plan (small amounts)

```bash
RELEASE_ALLOW_MAINNET=yes node scripts/release.mjs create-plan --contract <id> --source <merchant> \
  --token <token id> --amount <smallest units> --period 60
```

It prints `PLAN_ID=...`, the transaction and the fee. Check it:

```bash
node scripts/release.mjs status --contract <id> --reader <any G address> --plan <PLAN_ID>
```

The 60 second period is the shortest the contract allows, so the whole flow takes minutes. Keep the amount small, and **cancel right after the first charge** (step 7), because a 60 second plan is charged every minute until cancelled.

### 4. Start the keeper with a fee cap

On the machine that will run it. The signing key goes from the key store into the process environment; never write it to a file and never print it.

```bash
cd recurring-keeper && pnpm install --frozen-lockfile && pnpm build
KEEPER_SECRET_KEY="$(stellar keys secret <keeper identity>)" \
STELLAR_NETWORK=... STELLAR_RPC_URL=... STELLAR_NETWORK_PASSPHRASE=... CONTRACT_ID=<id> \
DB_PATH=./keeper.sqlite API_PORT=8080 POLL_INTERVAL_SECS=15 MAX_FEE_STROOPS=<cap> \
node dist/main.js
```

`MAX_FEE_STROOPS` is per transaction. A good starting value is about three times the measured cost of a charge (about 11,000 stroops on testnet). The keeper account's balance is the hard limit on total spending, so fund it with only the budget. Check it: `node scripts/check-health.ts --url http://127.0.0.1:8080 --expect charging` must print `healthy`.

### 5. Subscribe from a subscriber account you own

```bash
cd recurring-checkout && pnpm install --frozen-lockfile
E2E_RUN=1 E2E_ALLOW_MAINNET=yes E2E_NETWORK=... E2E_RPC_URL=... E2E_NETWORK_PASSPHRASE=... \
E2E_SIGN_IDENTITY=<subscriber> E2E_PLAN_ID=<PLAN_ID> E2E_CONTRACT_ID=<id> \
  pnpm exec vitest run scripts/e2e-subscribe.test.ts
```

(`E2E_ALLOW_MAINNET=yes` only for mainnet.) It prints `E2E_RESULT {...}` with the subscription id and the approve and subscribe transaction hashes. The first payment is made at this moment.

### 6. Wait for one charge

```bash
cd recurring-keeper
node scripts/wait-for-charge.ts --url http://127.0.0.1:8080 --sub <subscription id> --count 1 --timeout-secs 300
```

It prints `CHARGE_TX=...` when the keeper's first charge succeeded, and exits 1 on timeout (then stop and report).

### 7. Cancel and clean up

```bash
cd recurring-contracts
RELEASE_ALLOW_MAINNET=yes node scripts/release.mjs cancel --contract <id> --source <subscriber> --sub <subscription id>
RELEASE_ALLOW_MAINNET=yes node scripts/release.mjs revoke-approval --token <token id> --spender <id> --source <subscriber>
node scripts/release.mjs status --contract <id> --reader <any G address> --sub <subscription id>
```

Cancelling stops the charges; revoking sets the token approval back to 0, because **cancelling alone leaves the approval in place** (on the testnet rehearsal 110 XLM stayed approved until it was revoked). The status must show `Cancelled`.

### 8. Record and report

Write the contract id, the plan id, the subscription id and every transaction hash (public data only) in `docs/MAINNET.md`, and update each README's status to what is now true: what is live, on which network, and the audit status. Say whether the keeper is left running and under what budget and monitoring (`MAINNET_READINESS.md` section 6).

## If something fails

Each script exits non-zero and prints why. Nothing after a failed step is run. A failed deploy leaves no usable contract: do not use a contract id from a run that did not end with "On-chain hash equals the built hash". If the keeper fails or must stop, stop the process; subscriptions already made keep their due times, and anyone can still call `charge`. To stop charging entirely, cancel subscriptions and revoke approvals (`MAINNET_READINESS.md` section 5).
