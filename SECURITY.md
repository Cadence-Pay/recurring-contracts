# Security policy

## Status

**recurring-contracts has not been audited**, and neither has the Stellar Recurring contract it works with. It has only been run on the Stellar test network. Do not use it with money you cannot afford to lose.

## Reporting a vulnerability

Please report it **privately**, not in a public issue or pull request.

- Use the repository's **"Report a vulnerability"** button (the Security tab, GitHub private vulnerability reporting).
- If that is not available, open a public issue that says only that you have a security report and asks for a private way to send it. Do not put details in it.

Please include: what is affected, how to reproduce it, what you think the impact is, and the version or commit. Contract function and arguments, the network (testnet or local), and the transaction hash if there is one. Never include a real secret key or seed phrase; if one is needed to show the problem, use a throwaway testnet key.

## What to expect

The maintainers are volunteers. We will try to acknowledge a report within a few days, but we do not promise a response time, and there is no bug bounty. We will say when a problem is fixed and give credit if you want it.

## Scope

Of most interest for this package: the contract's logic: authorization, the charge schedule, storage and TTL handling, events, arithmetic.

Not a vulnerability in this project: problems in a wallet, in the Stellar network or its RPC servers, in a token contract a merchant chose, or caused by a leaked key that you published yourself. Reports about those are still welcome, but they belong with their owners.
