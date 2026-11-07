# TESTNET

Public addresses and contract IDs only. Secret keys live in the Stellar CLI key store (`~/.config/stellar/identity/`), never in this repo.

Network: Stellar testnet. Passphrase `Test SDF Network ; September 2015`, RPC `https://soroban-testnet.stellar.org`.

**Testnet resets on 2026-12-16 at 17:00 UTC (scheduled).** Accounts and contracts below will be wiped. Anything deployed before then has to be redeployed, and the identities re-funded with friendbot (`stellar keys fund <name> --network testnet`).

## Identities

Created 2026-11-05 with `stellar keys generate <name> --network testnet --fund`.

The names carry a `recurring-` prefix because an unrelated identity called `deployer` already exists in this machine's key store. It was not touched.

| Role | CLI identity | Public address |
|---|---|---|
| Deploys the contract | `recurring-deployer` | `GCZY6F6SY4Q5LO7FPP2UHT54TKZLRWACWIPB4SMO3DLVJKIREESXU7AN` |
| Runs the keeper | `recurring-keeper` | `GC7JVHSTSI2BO4KAJTHD5FRJZX4KIVCEMDYZ7RTS6GOA3J6FPHAI5CRL` |
| Merchant in rehearsals | `recurring-merchant` | `GAIPBT4WCBUALPT77TFYIK4LAHN67QNFM4OOTX3VEH4L6URNOUFJOTSQ` |
| Subscriber in rehearsals | `recurring-subscriber` | `GACJH7JXHQWIW66GHZFNMCK2I7WOZ562QAOF7B7PUENTAPRGC5RLW24A` |

## Contract and token

Deployed 2026-11-05 by `recurring-deployer` with `stellar contract deploy --network testnet --alias recurring-testnet`.

| Item | Value |
|---|---|
| Contract ID | `CCUXOGU3N5HWMLRDXC5TCQ3QDA23X3RJCDXWHMBIC4SFLD2IABQT465E` |
| CLI alias | `recurring-testnet` |
| Deploy transaction | `9d5fa7605df7e7be0cad4fb12045cbf46259d536519513a59937aaa1e97f832a` |
| Wasm hash (built = on-chain) | `dde9e953b6c94497f390ae99318a77f6f3937dacd260434648747c8183e68e3c` |
| Built from | recurring-contracts at commit `289f4bf` and earlier (the code is the same; only docs changed after the build) |
| Toolchain | rustc 1.99.0, stellar CLI 30.0.1, soroban-sdk 29.0.1 |
| Test token | native XLM Stellar Asset Contract: `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` |

The on-chain wasm was fetched back with `stellar contract fetch` and its SHA-256 matches the built file.

The rehearsal token is testnet XLM because it needs no issuer or trustlines. Amounts are in stroops (10,000,000 = 1 XLM).

## Live smoke test (2026-11-05)

One full lifecycle run by hand with the CLI. Plan 1: 1 XLM (10,000,000 stroops) every 60 seconds, merchant `recurring-merchant`.

| Step | Identity | Result | Transaction |
|---|---|---|---|
| `create_plan` | merchant | plan id 1 | `15ebeed62da010fce6dafd9e83d5dc61e0062fcf99eeedf4912cea1d358304d1` |
| token `approve` 5 XLM | subscriber | allowance set | `344bd247d7f06402d15c261d66812d7b8b803ff9d15f2e8cf9b13fb0bea52fa2` |
| `subscribe` | subscriber | sub id 1, first XLM paid | `c60e589893ab5c998ad8c9127bc65955ec9620dbdb49abcdafa98735837aae03` |
| `charge` before due | keeper | rejected, `Error(Contract, #7)` NotDue | simulation only |
| `charge` after 60 s | keeper | `charges: 1`, next due = charge time + 60 s | `4c420a6a176226239835cb6556ef6342b61510f435b873c8687bfbd0d9857bcb` |
| `charge` again, same period | keeper | rejected, `#7` NotDue | simulation only |
| `cancel` by an outsider | keeper | rejected, `#8` NotAuthorized | simulation only |
| `cancel` | subscriber | status `Cancelled` | `4da561dbc8328cd9a08123c42359dd51054c76065c63690371470466163dd85f` |
| `charge` after cancel | keeper | rejected, `#6` NotActive | simulation only |

Everything above behaved as the unit tests predict. The charge was submitted by the `keeper` identity, which is neither subscriber nor merchant, which shows `charge` is permissionless on a live network.

Not covered by this run: `deactivate_plan`, the `PaymentFailed` error on a live token, a second charge, and signing through a real wallet.

## Measured costs

See the table in [CONTRACT.md](CONTRACT.md). In short: `charge` cost 10,873 stroops (about 0.0011 XLM). Creating a plan or a subscription costs about 0.05 XLM each, mostly rent for 90 days.

## Allowance expiry limit

An `approve` with `live_until_ledger` up to the latest ledger plus 3,110,400 succeeded in simulation, and values beyond that failed (checked at latest+3,110,300 OK and latest+3,110,401 and above ERR). That matches the network `max_entry_ttl` of 3,110,400 ledgers, about 180 days.

## End-to-end run (2026-11-05)

Plan 2 (60 s, 1 XLM) and subscription 2 were created for the end-to-end run, charged twice by the keeper and cancelled. Transactions are listed in [DEMO.md](DEMO.md).
