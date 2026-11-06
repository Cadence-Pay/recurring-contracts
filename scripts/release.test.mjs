// Run with: node --test scripts/
import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  checkIdentityName,
  main,
  parseArgs,
  parseContractId,
  parseFeeCharged,
  parseTxHash,
  readNetwork,
  withNetworkFlags,
} from './release.mjs';

const TESTNET = {
  STELLAR_NETWORK: 'testnet',
  STELLAR_RPC_URL: 'https://soroban-testnet.stellar.org',
  STELLAR_NETWORK_PASSPHRASE: 'Test SDF Network ; September 2015',
};
const MAINNET = {
  STELLAR_NETWORK: 'mainnet',
  STELLAR_RPC_URL: 'https://rpc.example.com',
  STELLAR_NETWORK_PASSPHRASE: 'Public Global Stellar Network ; September 2015',
};

test('readNetwork accepts a testnet configuration', () => {
  assert.deepEqual(readNetwork(TESTNET), {
    label: 'testnet',
    rpcUrl: 'https://soroban-testnet.stellar.org',
    passphrase: 'Test SDF Network ; September 2015',
  });
});

test('readNetwork names every missing variable', () => {
  assert.throws(() => readNetwork({}), /STELLAR_NETWORK, STELLAR_RPC_URL, STELLAR_NETWORK_PASSPHRASE/);
});

test('readNetwork refuses a label whose passphrase does not match', () => {
  assert.throws(
    () => readNetwork({ ...MAINNET, STELLAR_NETWORK_PASSPHRASE: 'Test SDF Network ; September 2015' }),
    /does not match/,
  );
  assert.throws(
    () => readNetwork({ ...TESTNET, STELLAR_NETWORK_PASSPHRASE: 'Public Global Stellar Network ; September 2015' }),
    /does not match/,
  );
});

test('readNetwork refuses an RPC address that is not http(s)', () => {
  assert.throws(() => readNetwork({ ...TESTNET, STELLAR_RPC_URL: 'ftp://x' }), /http/);
});

test('readNetwork refuses mainnet unless the owner has set the guard', () => {
  assert.throws(() => readNetwork(MAINNET), /RELEASE_ALLOW_MAINNET=yes/);
  assert.throws(() => readNetwork({ ...MAINNET, RELEASE_ALLOW_MAINNET: 'true' }), /RELEASE_ALLOW_MAINNET=yes/);
  assert.equal(readNetwork({ ...MAINNET, RELEASE_ALLOW_MAINNET: 'yes' }).label, 'mainnet');
});

test('readNetwork allows a private network with its own passphrase', () => {
  const net = readNetwork({
    STELLAR_NETWORK: 'local',
    STELLAR_RPC_URL: 'http://localhost:8000/rpc',
    STELLAR_NETWORK_PASSPHRASE: 'Standalone Network ; February 2017',
  });
  assert.equal(net.label, 'local');
});

test('checkIdentityName accepts names and refuses secrets', () => {
  assert.equal(checkIdentityName('recurring-deployer'), 'recurring-deployer');
  assert.equal(checkIdentityName('alice_2.test'), 'alice_2.test');
  const secret = `S${'A'.repeat(55)}`;
  assert.throws(() => checkIdentityName(secret), /NAME of an identity/);
  assert.throws(() => checkIdentityName('kite urban word word word word word word word word word word'), /NAME of an identity/);
  assert.throws(() => checkIdentityName(''), /name of a Stellar CLI identity/);
  assert.throws(() => checkIdentityName(undefined), /name of a Stellar CLI identity/);
  assert.throws(() => checkIdentityName('a;rm -rf'), /NAME of an identity/);
});

test('a refused secret is never echoed back in the error', () => {
  const secret = `S${'B'.repeat(55)}`;
  try {
    checkIdentityName(secret);
    assert.fail('should have thrown');
  } catch (error) {
    assert.ok(!error.message.includes(secret));
  }
});

test('parseArgs reads --name value pairs and refuses anything else', () => {
  assert.deepEqual(parseArgs(['--source', 'alice', '--amount', '5']), { source: 'alice', amount: '5' });
  assert.throws(() => parseArgs(['source', 'alice']), /Unexpected argument/);
  assert.throws(() => parseArgs(['--source']), /needs a value/);
  assert.throws(() => parseArgs(['--source', '--amount', '5']), /needs a value/);
});

test('parseContractId takes the last contract id in the output', () => {
  const id = `C${'A'.repeat(55)}`;
  const other = `C${'B'.repeat(55)}`;
  assert.equal(parseContractId(`noise\n${other}\nmore\n${id}\n`), id);
  assert.equal(parseContractId('no id here'), undefined);
});

test('parseTxHash and parseFeeCharged read the CLI output', () => {
  const hash = 'a'.repeat(64);
  assert.equal(parseTxHash(`link https://stellar.expert/explorer/testnet/tx/${hash} ok`), hash);
  assert.equal(parseTxHash('nothing'), undefined);
  assert.equal(parseFeeCharged('| Fee Charged: 14858 | Refunded: 6561 |'), 14858);
  // The real table, as printed by the CLI: the total is the row that is exactly "Fee Charged".
  const table = [
    '| Inclusion Fee Charged: 100 | Non-Refundable Charged: 3277 | Refundable Charged: 11481 | Refunded: 6561 |',
    '| Inclusion Fee Charged: 100 | Resource Fee Charged: 14758 | Refunded: 6561 |',
    '| Fee Charged: 14858 | Refunded: 6561 |',
  ].join(String.fromCharCode(10));
  assert.equal(parseFeeCharged(table), 14858);
  assert.equal(parseFeeCharged('| Inclusion Fee Charged: 100 |'), undefined);
  assert.equal(parseFeeCharged('nothing'), undefined);
});

test('deploy refuses before building when the approved hash is missing or malformed', () => {
  assert.throws(() => main(['deploy', '--source', 'alice'], TESTNET), /--expect-hash/);
  assert.throws(() => main(['deploy', '--source', 'alice', '--expect-hash', 'abc'], TESTNET), /64-character/);
});

test('every command refuses a secret key given as the source, before any network call', () => {
  const secret = `S${'C'.repeat(55)}`;
  for (const args of [
    ['deploy', '--source', secret, '--expect-hash', 'a'.repeat(64)],
    ['create-plan', '--contract', `C${'A'.repeat(55)}`, '--source', secret, '--token', `C${'A'.repeat(55)}`, '--amount', '1', '--period', '60'],
    ['cancel', '--contract', `C${'A'.repeat(55)}`, '--source', secret, '--sub', '1'],
  ]) {
    assert.throws(() => main(args, TESTNET), /NAME of an identity/);
  }
});

test('mainnet is refused by every networked command without the guard', () => {
  for (const args of [
    ['deploy', '--source', 'alice', '--expect-hash', 'a'.repeat(64)],
    ['status', '--contract', `C${'A'.repeat(55)}`, '--reader', `G${'A'.repeat(55)}`],
  ]) {
    assert.throws(() => main(args, MAINNET), /RELEASE_ALLOW_MAINNET=yes/);
  }
});

test('create-plan validates its numbers before any network call', () => {
  const base = ['create-plan', '--contract', `C${'A'.repeat(55)}`, '--source', 'alice', '--token', `C${'A'.repeat(55)}`];
  assert.throws(() => main([...base, '--amount', '0', '--period', '60'], TESTNET), /--amount/);
  assert.throws(() => main([...base, '--amount', '-5', '--period', '60'], TESTNET), /--amount/);
  assert.throws(() => main([...base, '--amount', '5', '--period', 'abc'], TESTNET), /--period/);
});

test('network flags go before the -- separator, never after it', () => {
  const net = { rpcUrl: 'https://rpc.example', passphrase: 'P' };
  const invoke = ['contract', 'invoke', '--id', 'C1', '--', 'create_plan', '--amount', '5'];
  assert.deepEqual(withNetworkFlags(invoke, net, true), [
    'contract', 'invoke', '--id', 'C1',
    '--cost', '--rpc-url', 'https://rpc.example', '--network-passphrase', 'P',
    '--', 'create_plan', '--amount', '5',
  ]);
  assert.deepEqual(withNetworkFlags(['contract', 'fetch', '--id', 'C1'], net), [
    'contract', 'fetch', '--id', 'C1', '--rpc-url', 'https://rpc.example', '--network-passphrase', 'P',
  ]);
});
