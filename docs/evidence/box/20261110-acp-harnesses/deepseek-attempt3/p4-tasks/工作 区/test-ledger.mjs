import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import {
  LedgerError,
  compareCodePoints,
  parseJsonl,
  reduceLedger,
  serializeSummary,
} from './ledger.mjs';

const testDirectory = path.dirname(fileURLToPath(import.meta.url));
const cliPath = path.join(testDirectory, 'reconcile.mjs');

function makeTemporaryDirectory(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'ledger-test-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  return directory;
}

function runCli(args) {
  return spawnSync(process.execPath, [cliPath, ...args], {
    encoding: 'utf8',
    timeout: 15_000,
  });
}

function event(id, account, cents, kind) {
  return { id, account, cents, kind };
}

test('credits add and debits subtract using exact integer arithmetic', () => {
  const summary = reduceLedger([
    event('a', 'cash', 10, 'credit'),
    event('b', 'cash', 3, 'debit'),
    event('c', 'cash', 7, 'credit'),
  ]);

  assert.deepEqual(summary, {
    uniqueEvents: 3,
    duplicateEvents: 0,
    accounts: [{ account: 'cash', balanceCents: 14 }],
    netCents: 14,
  });
});

test('amounts near Number.MAX_SAFE_INTEGER remain exact', () => {
  const summary = reduceLedger([
    event('maximum', 'large', Number.MAX_SAFE_INTEGER, 'credit'),
    event('one', 'large', 1, 'debit'),
    event('two', 'large', 1, 'credit'),
  ]);

  assert.equal(summary.accounts[0].balanceCents, 9007199254740991);
  assert.equal(summary.netCents, 9007199254740991);
});

test('identical duplicate ids are counted and applied only once', () => {
  const summary = reduceLedger([
    event('same', 'alpha', 500, 'credit'),
    event('same', 'alpha', 500, 'credit'),
    event('same', 'alpha', 500, 'credit'),
    event('other', 'alpha', 25, 'debit'),
  ]);

  assert.deepEqual(summary, {
    uniqueEvents: 2,
    duplicateEvents: 2,
    accounts: [{ account: 'alpha', balanceCents: 475 }],
    netCents: 475,
  });
});

test('records with the same id but conflicting fields are invalid', () => {
  assert.throws(
    () =>
      reduceLedger([
        event('collision', 'alpha', 100, 'credit'),
        event('collision', 'alpha', 101, 'credit'),
      ]),
    (error) =>
      error instanceof LedgerError &&
      /conflicting records for id "collision"/.test(error.message),
  );

  assert.throws(
    () =>
      reduceLedger([
        event('collision', 'alpha', 100, 'credit'),
        event('collision', 'beta', 100, 'credit'),
      ]),
    LedgerError,
  );

  assert.throws(
    () =>
      reduceLedger([
        event('collision', 'alpha', 100, 'credit'),
        event('collision', 'alpha', 100, 'debit'),
      ]),
    LedgerError,
  );
});

test('accounts are sorted by Unicode code point, not UTF-16 code unit', () => {
  const summary = reduceLedger([
    event('1', '😀', 1, 'credit'),
    event('2', '\uE000', 1, 'credit'),
    event('3', '中', 1, 'credit'),
    event('4', 'é', 1, 'credit'),
    event('5', 'a', 1, 'credit'),
    event('6', 'A', 1, 'credit'),
  ]);

  assert.deepEqual(
    summary.accounts.map(({ account }) => account),
    ['A', 'a', 'é', '中', '\uE000', '😀'],
  );
  assert.equal(compareCodePoints('😀', '\uE000'), 1);
});

test('blank lines are ignored and CRLF input is accepted', () => {
  const text = [
    '',
    '{"id":"one","account":"a","cents":4,"kind":"credit"}',
    '   ',
    '\t',
    '{"id":"two","account":"a","cents":1,"kind":"debit"}',
    '',
  ].join('\r\n');

  const records = parseJsonl(text);
  assert.equal(records.length, 2);
  assert.deepEqual(reduceLedger(text), {
    uniqueEvents: 2,
    duplicateEvents: 0,
    accounts: [{ account: 'a', balanceCents: 3 }],
    netCents: 3,
  });
});

test('--account filtering limits accounts and net but not event counts', () => {
  const records = [
    event('1', 'alpha', 100, 'credit'),
    event('2', 'beta', 30, 'debit'),
    event('3', 'alpha', 5, 'debit'),
    event('1', 'alpha', 100, 'credit'),
  ];

  assert.deepEqual(reduceLedger(records, { account: 'alpha' }), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: 'alpha', balanceCents: 95 }],
    netCents: 95,
  });

  assert.deepEqual(reduceLedger(records, 'beta'), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: 'beta', balanceCents: -30 }],
    netCents: -30,
  });

  assert.deepEqual(reduceLedger(records, { account: 'missing' }), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [],
    netCents: 0,
  });
});

test('malformed and invalid records are rejected', () => {
  const invalidJsonl = [
    '{"id":',
    'null',
    '[]',
    '42',
    '"record"',
    '{}',
    '{"id":"","account":"a","cents":1,"kind":"credit"}',
    '{"id":3,"account":"a","cents":1,"kind":"credit"}',
    '{"id":"i","account":"","cents":1,"kind":"credit"}',
    '{"id":"i","account":9,"cents":1,"kind":"credit"}',
    '{"id":"i","account":"a","cents":-1,"kind":"credit"}',
    '{"id":"i","account":"a","cents":1.5,"kind":"credit"}',
    '{"id":"i","account":"a","cents":"1","kind":"credit"}',
    `{"id":"i","account":"a","cents":${
      Number.MAX_SAFE_INTEGER + 1
    },"kind":"credit"}`,
    '{"id":"i","account":"a","cents":1,"kind":"Credit"}',
    '{"id":"i","account":"a","cents":1,"kind":null}',
    '{"id":"i","account":"a","cents":1,"kind":"credit","extra":true} trailing',
  ];

  for (const text of invalidJsonl) {
    assert.throws(() => parseJsonl(text), LedgerError, text);
  }
});

test('the CLI writes the exact summary shape and creates parent directories', (t) => {
  const directory = makeTemporaryDirectory(t);
  const input = path.join(directory, 'input.jsonl');
  const output = path.join(directory, 'nested', 'deeper', 'result.json');

  fs.writeFileSync(
    input,
    [
      '{"id":"e1","account":"beta","cents":10,"kind":"credit"}',
      '{"id":"e2","account":"alpha","cents":7,"kind":"debit"}',
      '{"id":"e1","account":"beta","cents":10,"kind":"credit"}',
      '',
    ].join('\n'),
  );

  const result = runCli([input, output]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stderr, '');

  const text = fs.readFileSync(output, 'utf8');
  assert.match(
    text,
    /^\{"uniqueEvents":2,"duplicateEvents":1,"accounts":\[\{"account":"alpha","balanceCents":-7\},\{"account":"beta","balanceCents":10\}\],"netCents":3\}\n$/,
  );
  assert.deepEqual(JSON.parse(text), {
    uniqueEvents: 2,
    duplicateEvents: 1,
    accounts: [
      { account: 'alpha', balanceCents: -7 },
      { account: 'beta', balanceCents: 10 },
    ],
    netCents: 3,
  });
});

test('the CLI applies --account while keeping whole-input event counts', (t) => {
  const directory = makeTemporaryDirectory(t);
  const input = path.join(directory, 'input.jsonl');
  const output = path.join(directory, 'filtered.json');

  fs.writeFileSync(
    input,
    [
      '{"id":"1","account":"alpha","cents":100,"kind":"credit"}',
      '{"id":"2","account":"beta","cents":25,"kind":"credit"}',
      '{"id":"3","account":"alpha","cents":5,"kind":"debit"}',
      '{"id":"1","account":"alpha","cents":100,"kind":"credit"}',
    ].join('\n'),
  );

  const result = runCli([input, output, '--account', 'alpha']);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(fs.readFileSync(output, 'utf8')), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: 'alpha', balanceCents: 95 }],
    netCents: 95,
  });

  const equalsForm = runCli([input, output, '--account=beta']);
  assert.equal(equalsForm.status, 0, equalsForm.stderr);
  assert.deepEqual(JSON.parse(fs.readFileSync(output, 'utf8')), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: 'beta', balanceCents: 25 }],
    netCents: 25,
  });
});

test('invalid input fails without replacing an existing output', (t) => {
  const directory = makeTemporaryDirectory(t);
  const input = path.join(directory, 'bad.jsonl');
  const output = path.join(directory, 'existing.json');
  const sentinel = '{"keep":"me"}\n';
  fs.writeFileSync(input, '{"id":"bad"\n');
  fs.writeFileSync(output, sentinel);

  const result = runCli([input, output]);

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /reconcile: line 1: malformed JSON/);
  assert.equal(fs.readFileSync(output, 'utf8'), sentinel);
});

test('conflicting duplicate ids fail without creating or replacing output', (t) => {
  const directory = makeTemporaryDirectory(t);
  const input = path.join(directory, 'conflict.jsonl');
  const output = path.join(directory, 'existing.json');
  const missingOutput = path.join(directory, 'never-created.json');
  const sentinel = 'preserve this output';

  fs.writeFileSync(
    input,
    [
      '{"id":"x","account":"alpha","cents":1,"kind":"credit"}',
      '{"id":"x","account":"alpha","cents":2,"kind":"credit"}',
    ].join('\n'),
  );
  fs.writeFileSync(output, sentinel);

  const result = runCli([input, output]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /conflicting records for id "x"/);
  assert.equal(fs.readFileSync(output, 'utf8'), sentinel);

  const noOutputResult = runCli([input, missingOutput]);
  assert.notEqual(noOutputResult.status, 0);
  assert.equal(fs.existsSync(missingOutput), false);
});

test('the CLI rejects missing arguments, unknown options, and bad filters', (t) => {
  const directory = makeTemporaryDirectory(t);
  const input = path.join(directory, 'input.jsonl');
  fs.writeFileSync(
    input,
    '{"id":"1","account":"a","cents":1,"kind":"credit"}\n',
  );

  assert.notEqual(runCli([input]).status, 0);
  assert.notEqual(runCli([]).status, 0);
  assert.notEqual(
    runCli([input, path.join(directory, 'out.json'), '--bogus']).status,
    0,
  );
  assert.notEqual(
    runCli([input, path.join(directory, 'out.json'), '--account']).status,
    0,
  );
  assert.notEqual(
    runCli([input, path.join(directory, 'out.json'), '--account=']).status,
    0,
  );
});

test('empty input produces a zero summary', (t) => {
  const directory = makeTemporaryDirectory(t);
  const input = path.join(directory, 'empty.jsonl');
  const output = path.join(directory, 'empty.json');
  fs.writeFileSync(input, '\n  \n');

  const result = runCli([input, output]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(fs.readFileSync(output, 'utf8')), {
    uniqueEvents: 0,
    duplicateEvents: 0,
    accounts: [],
    netCents: 0,
  });
});

test('serializer emits unquoted integers and escapes account strings', () => {
  const text = serializeSummary({
    uniqueEvents: 1,
    duplicateEvents: 0,
    accounts: [{ account: 'quote"tab\t', balanceCents: -5 }],
    netCents: -5,
  });

  assert.equal(
    text,
    '{"uniqueEvents":1,"duplicateEvents":0,"accounts":[{"account":"quote\\"tab\\t","balanceCents":-5}],"netCents":-5}',
  );
  assert.equal(JSON.parse(text).accounts[0].account, 'quote"tab\t');
});
