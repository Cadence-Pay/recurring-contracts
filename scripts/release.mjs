#!/usr/bin/env node
/*
 * Release kit for the recurring contract. Plain Node, no dependencies. It only runs the Stellar CLI.
 *
 *   node scripts/release.mjs build
 *   node scripts/release.mjs deploy      --source <identity> --expect-hash <sha256> [--alias <name>] [--out <file>]
 *   node scripts/release.mjs create-plan --contract <id|alias> --source <identity> --token <id> --amount <n> --period <secs>
 *   node scripts/release.mjs cancel      --contract <id|alias> --source <identity> --sub <id>
 *   node scripts/release.mjs status      --contract <id|alias> [--plan <id>] [--sub <id>]
 *
 * The network is never built in. It comes from the environment, and all three are required:
 *   STELLAR_NETWORK              label: testnet or mainnet (or your own name for a private network)
 *   STELLAR_RPC_URL              the RPC server
 *   STELLAR_NETWORK_PASSPHRASE   must match the label if the label is testnet or mainnet
 *
 * Keys are never handled here. --source is the NAME of an identity in the Stellar CLI key store.
 * Something that looks like a secret key or a seed phrase is refused.
 *
 * Mainnet moves real value and cannot be undone. It is refused unless RELEASE_ALLOW_MAINNET=yes is
 * set in the environment of that one command.
 */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const WASM_PATH = 'target/wasm32v1-none/release/recurring_contracts.wasm';
const PACKAGE_DIR = join(dirname(fileURLToPath(import.meta.url)), '..');
const WASM_FILE = join(PACKAGE_DIR, WASM_PATH);

const KNOWN_PASSPHRASES = {
  testnet: 'Test SDF Network ; September 2015',
  mainnet: 'Public Global Stellar Network ; September 2015',
};

/** Read and check the network settings from the environment. Throws a readable Error. */
export function readNetwork(env) {
  const label = env.STELLAR_NETWORK;
  const rpcUrl = env.STELLAR_RPC_URL;
  const passphrase = env.STELLAR_NETWORK_PASSPHRASE;
  const missing = ['STELLAR_NETWORK', 'STELLAR_RPC_URL', 'STELLAR_NETWORK_PASSPHRASE'].filter(
    (name) => !env[name],
  );
  if (missing.length > 0) throw new Error(`Set ${missing.join(', ')} in the environment.`);
  if (!/^https?:\/\//.test(rpcUrl)) throw new Error('STELLAR_RPC_URL must start with http:// or https://');
  const expected = KNOWN_PASSPHRASES[label];
  if (expected !== undefined && expected !== passphrase) {
    throw new Error(`STELLAR_NETWORK_PASSPHRASE does not match the passphrase of ${label}.`);
  }
  if (label === 'mainnet' && env.RELEASE_ALLOW_MAINNET !== 'yes') {
    throw new Error(
      'Mainnet moves real value and cannot be undone. This kit refuses it unless RELEASE_ALLOW_MAINNET=yes is set for this command, which only the owner does.',
    );
  }
  return { label, rpcUrl, passphrase };
}

/** Refuse anything that is not the name of an identity: secret keys, seed phrases, odd text. */
export function checkIdentityName(name) {
  if (typeof name !== 'string' || name === '') throw new Error('--source needs the name of a Stellar CLI identity.');
  if (/^S[A-Z2-7]{55}$/.test(name) || /\s/.test(name) || !/^[A-Za-z0-9_.-]{1,64}$/.test(name)) {
    throw new Error(
      '--source must be the NAME of an identity in the Stellar CLI key store. A secret key or seed phrase was refused and was not used.',
    );
  }
  return name;
}

/** Command line options: --name value pairs. */
export function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (!a.startsWith('--')) throw new Error(`Unexpected argument: ${a}`);
    const key = a.slice(2);
    const value = argv[i + 1];
    if (value === undefined || value.startsWith('--')) throw new Error(`--${key} needs a value.`);
    out[key] = value;
    i++;
  }
  return out;
}

/** Contract id from the CLI's output: the last line that is a contract id. */
/** Where the network options go in a CLI call: before `--`. Exported so it can be tested. */
export function withNetworkFlags(args, net, cost = false) {
  const flags = [...(cost ? ['--cost'] : []), '--rpc-url', net.rpcUrl, '--network-passphrase', net.passphrase];
  const split = args.indexOf('--');
  return split === -1 ? [...args, ...flags] : [...args.slice(0, split), ...flags, ...args.slice(split)];
}

export function parseContractId(stdout) {
  const ids = stdout
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => /^C[A-Z2-7]{55}$/.test(l));
  return ids.at(-1);
}

/** Transaction hash from the CLI's explorer link, if there is one. */
export function parseTxHash(text) {
  return /\/tx\/([0-9a-f]{64})/.exec(text)?.[1];
}

/** Fee charged, in stroops, from the CLI's cost table, if there is one. */
export function parseFeeCharged(text) {
  // The CLI's table has rows "Inclusion Fee Charged", "Resource Fee Charged" and the total
  // "Fee Charged". Only the row that is exactly "Fee Charged" is the total.
  const m = /(?:^|\n)[^A-Za-z\n]*Fee Charged:\s*(\d+)/.exec(text);
  return m === null ? undefined : Number(m[1]);
}

export function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

/**
 * Run the Stellar CLI against the configured network. With `cost`, the CLI also prints a fee table
 * (to stderr), which is returned so the cost of each step can be reported.
 */
function stellar(args, net, { cost = false } = {}) {
  // Network options must come before `--`: after it, the CLI reads everything as arguments of the
  // contract function being called.
  const full = withNetworkFlags(args, net, cost);
  const r = spawnSync('stellar', full, { encoding: 'utf8' });
  if (r.error) throw new Error(`Could not run the Stellar CLI: ${r.error.message}`);
  if (r.status !== 0) throw new Error(`stellar ${args.slice(0, 3).join(' ')} failed:\n${r.stderr ?? ''}`);
  return { stdout: r.stdout ?? '', stderr: r.stderr ?? '' };
}

function say(line) {
  process.stdout.write(`${line}\n`);
}

function build() {
  say('Building the contract (stellar contract build)...');
  const r = spawnSync('stellar', ['contract', 'build'], { encoding: 'utf8', cwd: PACKAGE_DIR });
  if (r.error) throw new Error(`Could not run the Stellar CLI: ${r.error.message}`);
  if (r.status !== 0) throw new Error(`Build failed:
${r.stderr ?? ''}`);
  if (!existsSync(WASM_FILE)) throw new Error(`Expected ${WASM_PATH} after the build.`);
  const hash = sha256File(WASM_FILE);
  say(`Built ${WASM_PATH}`);
  say(`WASM_SHA256=${hash}`);
  return hash;
}

function deploy(net, opts, env) {
  const source = checkIdentityName(opts.source);
  const expected = (opts['expect-hash'] ?? '').toLowerCase();
  if (!/^[0-9a-f]{64}$/.test(expected)) {
    throw new Error('--expect-hash must be the 64-character SHA-256 you approved for this release.');
  }
  const built = build();
  if (built !== expected) {
    throw new Error(`Built hash ${built} is not the approved hash ${expected}. Nothing was deployed.`);
  }
  say(`Deploying to ${net.label} as identity "${source}"...`);
  const args = ['contract', 'deploy', '--wasm', WASM_FILE, '--source-account', source];
  if (opts.alias) args.push('--alias', opts.alias);
  const r = stellar(args, net, { cost: true });
  const contractId = parseContractId(r.stdout);
  if (contractId === undefined) throw new Error(`Could not read the contract id from the CLI output:\n${r.stdout}`);
  const txHash = parseTxHash(r.stdout + r.stderr);
  const fee = parseFeeCharged(r.stderr);
  say(`Deployed contract ${contractId}${fee === undefined ? '' : ` (fee charged ${fee} stroops)`}`);

  say('Fetching the code back from the network to compare it with the build...');
  const dir = mkdtempSync(join(tmpdir(), 'recurring-release-'));
  try {
    const out = join(dir, 'onchain.wasm');
    stellar(['contract', 'fetch', '--id', contractId, '--out-file', out], net);
    const onChain = sha256File(out);
    if (onChain !== built) {
      throw new Error(`ON-CHAIN HASH ${onChain} DIFFERS FROM THE BUILD ${built}. Do not use contract ${contractId}.`);
    }
    say(`On-chain hash equals the built hash: ${onChain}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }

  const record = {
    network: net.label,
    contractId,
    deployTxHash: txHash ?? null,
    wasmSha256: built,
    deployFeeStroops: fee ?? null,
    deployedAt: new Date().toISOString(),
    source,
  };
  if (opts.out) {
    writeFileSync(opts.out, `${JSON.stringify(record, null, 2)}\n`);
    say(`Wrote ${opts.out} (public data only).`);
  }
  say(`CONTRACT_ID=${contractId}`);
  return record;
}

function createPlan(net, opts) {
  const source = checkIdentityName(opts.source);
  if (!opts.contract) throw new Error('--contract is required.');
  if (!/^C[A-Z2-7]{55}$/.test(opts.token ?? '')) throw new Error('--token must be a token contract id (starts with C).');
  if (!/^[1-9]\d*$/.test(opts.amount ?? '')) throw new Error('--amount must be a positive whole number in the token\'s smallest unit.');
  if (!/^[1-9]\d*$/.test(opts.period ?? '')) throw new Error('--period must be a positive whole number of seconds.');
  const r = stellar(
    ['contract', 'invoke', '--id', opts.contract, '--source-account', source, '--', 'create_plan', '--merchant', source, '--token', opts.token, '--amount', opts.amount, '--period_secs', opts.period],
    net,
    { cost: true },
  );
  const planId = r.stdout.trim().split(/\r?\n/).at(-1)?.replace(/"/g, '');
  say(`Plan created: id ${planId}, transaction ${parseTxHash(r.stdout + r.stderr) ?? 'unknown'}, fee ${parseFeeCharged(r.stderr) ?? 'unknown'} stroops`);
  say(`PLAN_ID=${planId}`);
  return planId;
}

function cancel(net, opts) {
  const source = checkIdentityName(opts.source);
  if (!opts.contract || !/^[1-9]\d*$/.test(opts.sub ?? '')) throw new Error('--contract and --sub are required.');
  const r = stellar(
    ['contract', 'invoke', '--id', opts.contract, '--source-account', source, '--', 'cancel', '--caller', source, '--sub_id', opts.sub],
    net,
    { cost: true },
  );
  say(`Subscription ${opts.sub} cancelled, transaction ${parseTxHash(r.stdout + r.stderr) ?? 'unknown'}`);
}

function status(net, opts) {
  if (!opts.contract) throw new Error('--contract is required.');
  // A read needs some account to simulate from. Any public address will do; nothing is signed.
  if (!/^G[A-Z2-7]{55}$/.test(opts.reader ?? '')) {
    throw new Error('--reader must be any public account address (G...). Nothing is signed.');
  }
  const view = (fn, flag, value) =>
    stellar(
      ['contract', 'invoke', '--id', opts.contract, '--source-account', opts.reader, '--send=no', '--', fn, ...(flag ? [flag, value] : [])],
      net,
    ).stdout.trim();
  say(`plans: ${view('plan_count')}  subscriptions: ${view('sub_count')}`);
  if (opts.plan) say(`plan ${opts.plan}: ${view('get_plan', '--plan_id', opts.plan)}`);
  if (opts.sub) say(`subscription ${opts.sub}: ${view('get_subscription', '--sub_id', opts.sub)}`);
}

export function main(argv, env) {
  const [command, ...rest] = argv;
  if (command === 'build') return build();
  const opts = parseArgs(rest);
  const net = readNetwork(env);
  say(`Network: ${net.label}`);
  if (command === 'deploy') return deploy(net, opts, env);
  if (command === 'create-plan') return createPlan(net, opts);
  if (command === 'cancel') return cancel(net, opts);
  if (command === 'status') return status(net, opts);
  throw new Error('Usage: release.mjs build | deploy | create-plan | cancel | status (see the top of this file)');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main(process.argv.slice(2), process.env);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exit(1);
  }
}
