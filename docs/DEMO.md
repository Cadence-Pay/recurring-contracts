# DEMO

Notes on what has been run and seen, with dates. Only what actually happened is written here. The end-to-end run on testnet, with transaction hashes, is the second section.

## Phase 3: dashboard against the real keeper and testnet (2026-11-05)

### What was run

The built keeper (`recurring-keeper`, read only, no key) and the built dashboard (`recurring-dashboard`) were started on one machine against the deployed testnet contract `CCUXOGU3N5HWMLRDXC5TCQ3QDA23X3RJCDXWHMBIC4SFLD2IABQT465E` (see `TESTNET.md`).

- Keeper: `STELLAR_NETWORK=testnet`, public SDF testnet RPC, `API_PORT=18080`, `CORS_ORIGINS=http://127.0.0.1:3100`, `POLL_INTERVAL_SECS=5`.
- Dashboard: built with `NEXT_PUBLIC_KEEPER_URL=http://127.0.0.1:18080`, the testnet network label, passphrase, RPC URL and contract id, then `next start -p 3100`.

### What was checked and the result

| Check | Result |
|---|---|
| Keeper syncs from the live contract and `GET /health` reports `lastSyncAt` | Worked: 1 plan, 1 subscription copied |
| `GET /merchants/<merchant>/revenue` | `20000000` stroops over 2 payments: the first payment at subscribe plus the one charge made in the contract smoke test, which matches the chain |
| Dashboard pages `/`, `/plans`, `/plans/1`, `/revenue`, `/subscriptions/1` | All HTTP 200 with the right page titles; an unknown path gives 404 |
| Overview HTML | Skip link, `aria-label="Main"` navigation, `id="main"` landmark and the Connect wallet button are present, and no "not configured" message, so the settings validated |
| CORS preflight from the dashboard's origin to the keeper (`OPTIONS /plans`) | 204 with `access-control-allow-origin` set to the dashboard origin and `GET, OPTIONS` allowed |
| A request from another origin | No CORS header |

### What was not done

The pages were fetched with `curl`. They render their loading state on the server and fill in data in the browser, so this run proves the build, the configuration and the keeper-to-browser handshake, **not** what a person sees and does. In particular these were **not** tried by hand:

- connecting a real wallet (Freighter, xBull, ...) and the wrong-network warning with a real wallet,
- creating a plan, deactivating it and cancelling a subscription with real signatures,
- a keyboard-only walk through the pages and a screen reader.

Those steps are in the owner's action list (kept outside the package repositories). Automated stand-ins exist: 211 dashboard tests cover the forms, the states, the wallet adapter with a fake kit, the transaction service with a fake client, the contrast of the colour tokens and a mechanical accessibility audit of every view.

### Known issues so far

- The `charges` table of the keeper only has records for charges the keeper itself made. The live subscription 1 was charged with the CLI, so its charges page is empty while its revenue includes that charge. This is by design and is explained on the charges page.
- Testnet resets on 2026-12-16 and will remove the contract and the accounts used above.


## Phase 5: end-to-end run on testnet (2026-11-05)

Steps to repeat it: [recurring-checkout/scripts/e2e-testnet.md](https://github.com/Cadence-Pay/recurring-checkout/blob/main/scripts/e2e-testnet.md).

### What was run

A 60 second plan was created, a subscriber subscribed through the **widget's real flow code**, the **built keeper** (with its signing key, a 1,000,000 stroop fee cap, polling every 5 s) charged twice on its own, and the subscriber cancelled. Everything is on the Stellar testnet, contract `CCUXOGU3N5HWMLRDXC5TCQ3QDA23X3RJCDXWHMBIC4SFLD2IABQT465E`, token = testnet XLM, 1 XLM per period.

The "wallet" was the Stellar CLI signing with a key held under a name in its key store (`recurring-subscriber`), so no secret key was seen, printed or stored. The keeper's key was passed from the key store into its process environment without being printed; its log was checked and contains no secret.

### Transactions

| Step | Who | Result | Transaction | Time (UTC) |
|---|---|---|---|---|
| Create plan 2 (60 s, 1 XLM) | merchant, CLI | plan id 2 | `700892aa3ed62b978a042ed257004f57bc985be6fa5762c5fa94a5fc1cc8785a` | |
| Approve (widget flow) | subscriber | cap 12 XLM = 12 charges | `1863e5347291ea987c88026e1ec600f9c6fdc256788d1f72a184b17cab1fcee5` | |
| Subscribe (widget flow) | subscriber | subscription id 2, first XLM paid | `f49e5c4c5bc8699d416714c5b9c2c72899bfe567fd909e8f2f4af13192e7db5a` | |
| Charge 1 | keeper (automatic) | `succeeded`, 0 retries | `208f42da7d2c763369c91d03464b9b70e0c844080347718a14dddeab82af9726` | due 21:50:11, recorded 21:50:35 |
| Charge 2 | keeper (automatic) | `succeeded` after 2 retries | `3dbcde65cb0dcb4cc7f296d3a30dd01ac933216545a16beebb97b2c5f24a1b0a` | due 21:51:11, recorded 21:54:07 |
| Cancel | subscriber, CLI | status `Cancelled` | `cde1bb1313e45ddda41448e47103ea975642a45aac0eb96cefdd001100b8845b` | |
| Charge after cancel | anyone | refused, `Error(Contract, #6)` NotActive | simulation only | |

Look any of them up at `https://stellar.expert/explorer/testnet/tx/<hash>`.

### What the checks showed

- The widget flow ran in the expected order: connecting, checking the balance, approving, subscribing. The approval was 12 XLM (12 charges), as the widget states.
- The keeper found the subscription due and charged it without any help. After the cancel the on-chain subscription read `charges: 2`, `Cancelled`, and the keeper's API showed the same after its next sync.
- The merchant's revenue figure from the API was `50000000` stroops over 5 payments: subscription 1 (first payment + 1 charge) plus subscription 2 (first payment + 2 charges). That matches the chain.
- Charge 2 took about 3 minutes instead of 1: the public RPC **timed out twice** (`RPC_TIMEOUT`, 15 s limit at the time), the keeper waited 30 s and then 60 s as designed, and then succeeded. No `charge.failed` event, no double charge. The default timeout was raised to 30 s afterwards.

### Known issues

- **No browser, no real wallet.** The wallet in this run was the CLI. The dashboard's screens and the widget's look were not seen by a person in a browser; see the owner's action list (kept outside the package repositories). No screenshots exist for that reason.
- The dashboard was not used in this run: plans were created, and the subscription cancelled, with the CLI. The dashboard's own create and cancel paths are covered by unit tests and a smoke run, not by this run.
- The public testnet RPC is sometimes slow (timeouts above). Mainnet needs a provider with a key and a longer timeout budget.
- Testnet is wiped on 2026-12-16. The contract, plans and accounts above disappear then.
- Revenue and the keeper's charge table differ by design: charges made by another party are counted in revenue but have no keeper record.
- The cancel does not remove the token approval (12 XLM cap) from the subscriber's account. It stays until it expires or the subscriber removes it.


## Phase 5.4: network switch test (2026-11-05)

Goal: show that every network value comes from configuration and nothing network-specific is written in the code, and that a mainnet configuration can be tried safely in read-only mode.

### Static check

A search of the production source of all four packages (tests, generated bindings and docs excluded) for RPC URLs, passphrases, friendbot, Horizon, account ids and contract ids found only:

- display labels: the widget turns a passphrase into the word "Testnet" or "Mainnet" for a message, and the dashboard builds a link to a public block explorer from the network label;
- a cross-check in the keeper's and the dashboard's configuration that refuses a network name whose passphrase does not match (`mainnet` with the testnet passphrase is an error);
- doc comments.

None of them is used as the value to connect to or sign for. The network, RPC URL, passphrase and contract id are read from the environment (keeper, dashboard) or from props (widget).

### Mainnet in read-only mode

The built keeper was started with `STELLAR_NETWORK=mainnet`, the public mainnet RPC, the mainnet passphrase and **no signing key**. Because no recurring contract exists on mainnet, the contract id given was Circle's USDC token contract on mainnet (a real contract that is not ours).

- `GET /health` reported `"network":"mainnet"` and `"readOnly":true`. The startup log printed `"keeperSecretKey":"unset"`.
- The keeper reached the mainnet RPC. The token contract refused our read calls (it has no `plan_count`), so each pass was logged as failed and the next pass ran. It did not crash.
- Nothing was signed or submitted (no `charged`, `submit` or `signAndSend` in the log), and none could be: there was no key.
- Starting it with the mainnet name and the **testnet** passphrase was refused at startup with exit code 2 and the message "STELLAR_NETWORK_PASSPHRASE does not match the passphrase of mainnet".

### What this does not show

No recurring contract exists on mainnet, so the mainnet run could not read real plans. The dashboard and the widget were not started against mainnet. They are covered by configuration unit tests (the passphrase cross-check, props validation) and use the same config path. Deploying to mainnet is Phase 7 and needs a funded mainnet identity supplied by the user.

### Found and fixed during this test

The RPC's refusal came back as a multi-page diagnostic dump that was logged and stored in full. Error text is now cut to 400 characters (the start, where the cause is, is kept).


## Phase 7.2: release kit rehearsal on testnet (2026-11-06)

The whole release sequence from [RELEASE.md](RELEASE.md) was run on testnet with the release kit's own scripts, against a **new** contract deployed by the kit, with a 60 second plan of 1 XLM. It is the same sequence that would be run on mainnet; only the environment variables differ.

| Step | Script | Result | Transaction |
|---|---|---|---|
| Wrong approved hash | `release.mjs deploy --expect-hash fff...` | refused before anything was sent, exit 1 | none |
| Mainnet without the guard | `release.mjs status` with the mainnet passphrase | refused, exit 1 | none |
| Deploy with the approved hash | `release.mjs deploy` | contract `CBG4BXKZTD2OCTEY4JQGSIVEDCOS7WXIZHGUMNIWCIVRGOCYPLU6UKWV`, on-chain hash equals the build (`dde9e953...e68e3c`) | `53ba373e770bfb3de6ddbf3b249e069295468979d6b7fdb10bd80f1698df8f08` |
| First plan | `release.mjs create-plan` | plan 1, fee 523,508 stroops | `b36b80d1970f323cf86e71c76f7e2b5ff09bb815bc33b83ce55e7a510c748f8b` |
| Read it back | `release.mjs status` | 1 plan, the right terms | read only |
| Keeper started, health | `check-health.ts --expect charging` | `healthy` | |
| Subscribe (widget flow) | `e2e-subscribe.test.ts` | subscription 1, approval cap 12 XLM | approve `75f7a420eddc623a3b004ef8ea8ecbd973aec39efcd44cb57d412571c0b82afa`, subscribe `6d4e81e0e0c1c54148cf6250450b4ad779a8448a4cff37340a24ec2b29f57051` |
| One charge by the keeper | `wait-for-charge.ts` | 1 charge succeeded | `53eda7299a9821cb674d79bf41eab05d58c938109b21f2d20dd756fa227ca193` |
| Health again | `check-health.ts` | `healthy` | |
| Cancel | `release.mjs cancel` | status `Cancelled`, `charges: 1`, the keeper's API agreed | `56f39e5fba03920267c8da5769bb958b927b2007a949115d1fed7fb02fb62d91` |
| Revoke the approval | `release.mjs revoke-approval` | allowance went from 110 XLM to 0 | `4c753eb0fdef8b3ffc4a9f3df2b00cb08d9903f3926228315e601ec52aca5dbb` |

The keeper's log was checked for secrets afterwards and contains none. The keeper was started with its key passed from the CLI key store into its environment without being printed.

### Found and fixed by the rehearsal

- The kit appended the network options after the `--` separator, where the CLI reads them as arguments of the contract function, so `create-plan` and `status` failed. Network options now go before `--` (with a test).
- The kit read the first "Fee Charged" number in the CLI table (the inclusion fee, 100) instead of the total. The deploy record written before the fix (`deployFeeStroops: 100`) is therefore wrong; the figures printed by later steps are right, and the parser is tested against the real table.
- The subscribe script's on-switch was named `E2E_TESTNET`, which would have been misleading in a mainnet command. It is now `E2E_RUN`.
- Cancelling leaves the token approval in place (110 XLM stayed approved). The kit gained `revoke-approval`, and the runbook ends with it.

### What the rehearsal does not show

It was testnet only, with the Stellar CLI signing as the subscriber (a mainnet release signs the same way with an identity the owner provides). No mainnet RPC provider, mainnet token, mainnet fees or mainnet limits were involved, and the fee and limit figures must be re-measured there.
