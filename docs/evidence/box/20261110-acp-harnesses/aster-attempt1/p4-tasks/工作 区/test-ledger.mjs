import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { LedgerError, compareByCodePoint, parseLedgerText, reduceLedger } from './ledger.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const CLI = path.join(here, 'reconcile.mjs');

function makeTempDir(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ledger-test-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  return dir;
}

function writeJsonl(file, lines) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, lines.join('\n'));
  return file;
}

function cli(args) {
  return spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8' });
}

function event(id, account, cents, kind) {
  return { id, account, cents, kind };
}

test('reducer totals credits and debits with integer arithmetic', () => {
  const result = reduceLedger([
    event('e1', 'cash', 1000, 'credit'),
    event('e2', 'cash', 250, 'debit'),
    event('e3', 'bank', 3, 'credit'),
  ]);
  assert.deepEqual(result, {
    uniqueEvents: 3,
    duplicateEvents: 0,
    accounts: [
      { account: 'bank', balanceCents: 3 },
      { account: 'cash', balanceCents: 750 },
    ],
    netCents: 753,
  });
});

test('identical duplicate ids are counted once and not double-counted', () => {
  const result = reduceLedger([
    event('e1', 'cash', 500, 'credit'),
    event('e1', 'cash', 500, 'credit'),
    event('e1', 'cash', 500, 'credit'),
    event('e2', 'cash', 100, 'debit'),
  ]);
  assert.equal(result.uniqueEvents, 2);
  assert.equal(result.duplicateEvents, 2);
  assert.deepEqual(result.accounts, [{ account: 'cash', balanceCents: 400 }]);
  assert.equal(result.netCents, 400);
});

test('records sharing an id but disagreeing are invalid', () => {
  assert.throws(
    () =>
      reduceLedger([
        event('e1', 'cash', 500, 'credit'),
        event('e1', 'cash', 500, 'debit'),
      ]),
    (error) => error instanceof LedgerError && /conflicting records/.test(error.message),
  );
});

test('integer arithmetic keeps exact cents at safe-integer limits', () => {
  const max = Number.MAX_SAFE_INTEGER;
  const result = reduceLedger([
    event('a', 'big', max, 'credit'),
    event('b', 'big', max, 'debit'),
    event('c', 'big', 1, 'credit'),
  ]);
  assert.deepEqual(result.accounts, [{ account: 'big', balanceCents: 1 }]);
  assert.equal(result.netCents, 1);
});

test('non-integer and unsafe cent values are rejected', () => {
  for (const cents of [1.5, 0.1 + 0.2, -1, 2 ** 53, Number.MAX_SAFE_INTEGER + 2, '10', null]) {
    assert.throws(
      () => reduceLedger([event('e1', 'cash', cents, 'credit')]),
      LedgerError,
      `cents ${String(cents)} should be rejected`,
    );
  }
});

test('invalid ids, accounts, kinds and shapes are rejected', () => {
  const bad = [
    { account: 'cash', cents: 1, kind: 'credit' },
    event('', 'cash', 1, 'credit'),
    event('e1', '', 1, 'credit'),
    event('e1', 'cash', 1, 'transfer'),
    event('e1', 'cash', 1, undefined),
    [],
    'not an object',
    null,
  ];
  for (const record of bad) {
    assert.throws(() => reduceLedger([record]), LedgerError, JSON.stringify(record));
  }
});

test('malformed JSON lines are rejected with a line number', () => {
  assert.throws(
    () => parseLedgerText('{"id":"a","account":"x","cents":1,"kind":"credit"}\n{oops}\n'),
    (error) => error instanceof LedgerError && /line 2/.test(error.message),
  );
});

test('blank lines are ignored and duplicates are counted across them', () => {
  const text = [
    '',
    '{"id":"a","account":"x","cents":10,"kind":"credit"}',
    '   ',
    '{"id":"a","account":"x","cents":10,"kind":"credit"}',
    '',
    '{"id":"b","account":"y","cents":4,"kind":"debit"}',
    '',
  ].join('\n');
  const result = reduceLedger(parseLedgerText(text));
  assert.equal(result.uniqueEvents, 2);
  assert.equal(result.duplicateEvents, 1);
  assert.deepEqual(result.accounts, [
    { account: 'x', balanceCents: 10 },
    { account: 'y', balanceCents: -4 },
  ]);
  assert.equal(result.netCents, 6);
});

test('accounts sort by Unicode code point, not UTF-16 code unit', () => {
  const accounts = ['\u{1F600}', '\u{1D11E}', 'a', 'Z', 'ä'];
  const result = reduceLedger(accounts.map((account, i) => event(`e${i}`, account, 1, 'credit')));
  assert.deepEqual(
    result.accounts.map((entry) => entry.account),
    ['Z', 'a', 'ä', '\u{1D11E}', '\u{1F600}'],
  );
  assert.equal(compareByCodePoint('\u{1F600}', '\u{1D11E}'), 1);
  assert.equal(compareByCodePoint('ab', 'abc'), -1);
  assert.equal(compareByCodePoint('x', 'x'), 0);
});

test('--account filters balances and net but not event counts', () => {
  const records = [
    event('e1', 'cash', 500, 'credit'),
    event('e2', 'cash', 200, 'debit'),
    event('e3', 'bank', 70, 'credit'),
    event('e3', 'bank', 70, 'credit'),
  ];
  const result = reduceLedger(records, { account: 'cash' });
  assert.equal(result.uniqueEvents, 3);
  assert.equal(result.duplicateEvents, 1);
  assert.deepEqual(result.accounts, [{ account: 'cash', balanceCents: 300 }]);
  assert.equal(result.netCents, 300);

  const missing = reduceLedger(records, { account: 'absent' });
  assert.deepEqual(missing.accounts, []);
  assert.equal(missing.netCents, 0);
  assert.equal(missing.uniqueEvents, 3);
});

test('CLI writes the exact JSON shape and creates parent directories', (t) => {
  const dir = makeTempDir(t);
  const input = writeJsonl(path.join(dir, 'in.jsonl'), [
    '{"id":"e1","account":"现金","cents":1000,"kind":"credit"}',
    '{"id":"e2","account":"现金","cents":250,"kind":"debit"}',
    '{"id":"e3","account":"Bank","cents":7,"kind":"credit"}',
    '',
  ]);
  const output = path.join(dir, 'nested', 'deeper', 'out.json');

  const result = cli([input, output]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.readFileSync(output, 'utf8').trim(), JSON.stringify({
    uniqueEvents: 3,
    duplicateEvents: 0,
    accounts: [
      { account: 'Bank', balanceCents: 7 },
      { account: '现金', balanceCents: 750 },
    ],
    netCents: 757,
  }));
});

test('CLI deduplicates and supports --account filtering', (t) => {
  const dir = makeTempDir(t);
  const input = writeJsonl(path.join(dir, 'in.jsonl'), [
    '{"id":"e1","account":"cash","cents":500,"kind":"credit"}',
    '{"id":"e1","account":"cash","cents":500,"kind":"credit"}',
    '{"id":"e2","account":"bank","cents":90,"kind":"credit"}',
  ]);
  const output = path.join(dir, 'out.json');

  const result = cli([input, output, '--account', 'cash']);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(fs.readFileSync(output, 'utf8')), {
    uniqueEvents: 2,
    duplicateEvents: 1,
    accounts: [{ account: 'cash', balanceCents: 500 }],
    netCents: 500,
  });
});

test('CLI fails on invalid input and preserves an existing output', (t) => {
  const dir = makeTempDir(t);
  const output = path.join(dir, 'out.json');
  fs.writeFileSync(output, 'PREVIOUS CONTENT');

  const cases = [
    ['malformed json', ['{"id":"e1","account":"a","cents":1,"kind":"credit"}', '{nope}']],
    ['conflicting id', [
      '{"id":"e1","account":"a","cents":1,"kind":"credit"}',
      '{"id":"e1","account":"a","cents":2,"kind":"credit"}',
    ]],
    ['negative cents', ['{"id":"e1","account":"a","cents":-5,"kind":"credit"}']],
    ['bad kind', ['{"id":"e1","account":"a","cents":5,"kind":"transfer"}']],
    ['empty id', ['{"id":"","account":"a","cents":5,"kind":"credit"}']],
  ];

  for (const [label, lines] of cases) {
    const input = writeJsonl(path.join(dir, `${label.replace(/\W+/g, '-')}.jsonl`), lines);
    const result = cli([input, output]);
    assert.notEqual(result.status, 0, `${label}: expected nonzero exit`);
    assert.match(result.stderr, /error:/, `${label}: expected stderr message`);
    assert.equal(fs.readFileSync(output, 'utf8'), 'PREVIOUS CONTENT', `${label}: output was replaced`);
  }
});

test('CLI does not create output when input is invalid or missing', (t) => {
  const dir = makeTempDir(t);
  const badInput = writeJsonl(path.join(dir, 'bad.jsonl'), ['{"id":"e1"}']);
  const output = path.join(dir, 'fresh', 'out.json');

  const invalid = cli([badInput, output]);
  assert.notEqual(invalid.status, 0);
  assert.equal(fs.existsSync(output), false);

  const missing = cli([path.join(dir, 'absent.jsonl'), output]);
  assert.notEqual(missing.status, 0);
  assert.equal(fs.existsSync(output), false);
});

test('CLI rejects bad usage with nonzero exit', (t) => {
  const dir = makeTempDir(t);
  const input = writeJsonl(path.join(dir, 'in.jsonl'), [
    '{"id":"e1","account":"a","cents":1,"kind":"credit"}',
  ]);
  const output = path.join(dir, 'out.json');

  for (const args of [
    [],
    [input],
    [input, output, '--account'],
    [input, output, '--bogus'],
    [input, output, '--account', 'a', '--account', 'b'],
    [input, output, 'extra'],
  ]) {
    const result = cli(args);
    assert.notEqual(result.status, 0, `args ${JSON.stringify(args)} should fail`);
    assert.equal(fs.existsSync(output), false);
  }
});
