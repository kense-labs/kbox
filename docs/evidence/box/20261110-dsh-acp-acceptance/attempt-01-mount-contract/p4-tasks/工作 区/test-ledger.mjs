import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import reduceLedger, {
  compareByCodePoint,
  formatReport,
  LedgerError,
  parseJsonl,
  reduce,
  reduceEvents,
  reducer,
} from './ledger.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const cliPath = join(here, 'reconcile.mjs');
const root = mkdtempSync(join(tmpdir(), 'ledger-tests-'));
let directoryCounter = 0;
let fileCounter = 0;

after(() => {
  rmSync(root, { recursive: true, force: true });
});

function testDirectory() {
  directoryCounter += 1;
  const directory = join(root, `case-${directoryCounter}`);
  mkdirSync(directory, { recursive: true });
  return directory;
}

function writeInput(content) {
  fileCounter += 1;
  const directory = testDirectory();
  const input = join(directory, `input-${fileCounter}.jsonl`);
  writeFileSync(input, content);
  return { directory, input };
}

function runCli(args) {
  return spawnSync(process.execPath, [cliPath, ...args], { encoding: 'utf8' });
}

function event(id, account, cents, kind) {
  return { id, account, cents, kind };
}

function jsonl(...events) {
  return `${events.map((item) => JSON.stringify(item)).join('\n')}\n`;
}

test('exports the reducer under its documented and convenient names', () => {
  assert.equal(reduce, reduceLedger);
  assert.equal(reduceEvents, reduceLedger);
  assert.equal(reducer, reduceLedger);
});

test('deduplicates identical ids and counts only unique events in balances', () => {
  const report = reduceLedger([
    event('a', 'alpha', 100, 'credit'),
    event('b', 'alpha', 30, 'debit'),
    event('a', 'alpha', 100, 'credit'),
    event('a', 'alpha', 100, 'credit'),
    event('c', 'beta', 7, 'credit'),
  ]);

  assert.equal(report.uniqueEvents, 3);
  assert.equal(report.duplicateEvents, 2);
  assert.deepEqual(report.accounts, [
    { account: 'alpha', balanceCents: 70 },
    { account: 'beta', balanceCents: 7 },
  ]);
  assert.equal(report.netCents, 77);
});

test('rejects conflicting records that share an id', () => {
  assert.throws(
    () => reduceLedger([
      event('same', 'alpha', 10, 'credit'),
      event('same', 'alpha', 11, 'credit'),
    ]),
    (error) => error instanceof LedgerError && error.code === 'CONFLICTING_ID',
  );

  assert.throws(
    () => reduceLedger([
      event('same', 'alpha', 10, 'credit'),
      event('same', 'alpha', 10, 'debit'),
    ]),
    (error) => error instanceof LedgerError && error.code === 'CONFLICTING_ID',
  );
});

test('rejects a conflict even when its account would be filtered out', () => {
  assert.throws(
    () => reduceLedger([
      event('ok', 'alpha', 10, 'credit'),
      event('bad', 'beta', 1, 'credit'),
      event('bad', 'beta', 2, 'credit'),
    ], { account: 'alpha' }),
    (error) => error instanceof LedgerError && error.code === 'CONFLICTING_ID',
  );
});

test('uses exact integer arithmetic and preserves unsafe sums as BigInt', () => {
  const max = Number.MAX_SAFE_INTEGER;

  const exactSafe = reduceLedger([
    event('credit', 'a', max, 'credit'),
    event('debit', 'a', 1, 'debit'),
  ]);
  assert.equal(exactSafe.netCents, max - 1);
  assert.equal(exactSafe.accounts[0].balanceCents, max - 1);

  const beyondSafe = reduceLedger([
    event('one', 'a', max, 'credit'),
    event('two', 'a', max, 'credit'),
    event('three', 'a', 1, 'credit'),
  ]);
  assert.equal(beyondSafe.netCents, 18014398509481983n);
  assert.equal(beyondSafe.accounts[0].balanceCents, 18014398509481983n);

  const exactText = formatReport(beyondSafe);
  assert.match(exactText, /"balanceCents":18014398509481983\}/);
  assert.match(exactText, /"netCents":18014398509481983\}/);
});

test('sorts accounts by Unicode code point, not UTF-16 code unit', () => {
  // Default JS string comparison sees the surrogate 0xD800, so it gets this
  // pair backwards relative to real Unicode code point order.
  assert.ok('\u{10000}' < '\uE000');
  assert.ok(compareByCodePoint('\uE000', '\u{10000}') < 0);

  const names = ['\u{10000}', '账户甲', 'a', '\uE000', 'é', 'Z'];
  const report = reduceLedger(names.map((name, index) => (
    event(`id-${index}`, name, index + 1, 'credit')
  )));

  assert.deepEqual(
    report.accounts.map(({ account }) => account),
    ['Z', 'a', 'é', '账户甲', '\uE000', '\u{10000}'],
  );
});

test('filters accounts and net but keeps global event counts', () => {
  const report = reduceLedger([
    event('a', 'alpha', 100, 'credit'),
    event('b', 'beta', 40, 'debit'),
    event('c', 'beta', 5, 'credit'),
    event('d', 'gamma', 9, 'credit'),
    event('a', 'alpha', 100, 'credit'),
  ], { account: 'beta' });

  assert.equal(report.uniqueEvents, 4);
  assert.equal(report.duplicateEvents, 1);
  assert.deepEqual(report.accounts, [{ account: 'beta', balanceCents: -35 }]);
  assert.equal(report.netCents, -35);

  const missing = reduceLedger([
    event('a', 'alpha', 100, 'credit'),
  ], { account: 'nobody' });
  assert.equal(missing.uniqueEvents, 1);
  assert.deepEqual(missing.accounts, []);
  assert.equal(missing.netCents, 0);
});

test('parses BOM, CRLF, and blank or whitespace-only lines', () => {
  const text = `\uFEFF${JSON.stringify(event('a', 'alpha', 10, 'credit'))}\r\n`
    + '\r\n'
    + '   \t  \n'
    + `${JSON.stringify(event('b', 'beta', 2, 'debit'))}\r\n`;

  const report = reduceLedger(text);
  assert.equal(report.uniqueEvents, 2);
  assert.equal(report.netCents, 8);
  assert.equal(parseJsonl(text).length, 2);
});

test('CLI writes the exact report shape and creates parent directories', () => {
  const { directory, input } = writeInput(jsonl(
    event('a', '账户甲', 1000, 'credit'),
    event('b', 'alpha', 250, 'debit'),
    event('a', '账户甲', 1000, 'credit'),
  ));
  const output = join(directory, 'nested', 'deeper', 'report.json');

  const result = runCli([input, output]);
  assert.equal(result.status, 0, result.stderr);

  const text = readFileSync(output, 'utf8');
  assert.match(text, /^\{.*\}\n$/s);
  const report = JSON.parse(text);
  assert.deepEqual(Object.keys(report), [
    'uniqueEvents',
    'duplicateEvents',
    'accounts',
    'netCents',
  ]);
  assert.deepEqual(report, {
    uniqueEvents: 2,
    duplicateEvents: 1,
    accounts: [
      { account: 'alpha', balanceCents: -250 },
      { account: '账户甲', balanceCents: 1000 },
    ],
    netCents: 750,
  });
});

test('CLI --account filters accounts and net without changing event counts', () => {
  const { directory, input } = writeInput(jsonl(
    event('a', 'alpha', 100, 'credit'),
    event('b', 'beta', 60, 'credit'),
    event('c', 'beta', 10, 'debit'),
    event('b', 'beta', 60, 'credit'),
  ));
  const output = join(directory, 'report.json');

  const result = runCli([input, output, '--account', 'beta']);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(readFileSync(output, 'utf8')), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: 'beta', balanceCents: 50 }],
    netCents: 50,
  });

  const equalsForm = runCli([input, join(directory, 'report-equals.json'), '--account=alpha']);
  assert.equal(equalsForm.status, 0, equalsForm.stderr);
  assert.deepEqual(
    JSON.parse(readFileSync(join(directory, 'report-equals.json'), 'utf8')).accounts,
    [{ account: 'alpha', balanceCents: 100 }],
  );
});

test('CLI filters a Unicode account name while retaining global counts', () => {
  const { directory, input } = writeInput(jsonl(
    event('a', 'alpha', 100, 'credit'),
    event('b', '账户甲', 40, 'debit'),
    event('c', '账户甲', 5, 'credit'),
    event('b', '账户甲', 40, 'debit'),
    event('d', 'beta', 7, 'credit'),
  ));
  const output = join(directory, 'filtered.json');

  const result = runCli([input, output, '--account', '账户甲']);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(readFileSync(output, 'utf8')), {
    uniqueEvents: 4,
    duplicateEvents: 1,
    accounts: [{ account: '账户甲', balanceCents: -35 }],
    netCents: -35,
  });
});

test('CLI reports exact decimal values for sums beyond safe integers', () => {
  const max = Number.MAX_SAFE_INTEGER;
  const { directory, input } = writeInput(jsonl(
    event('one', 'huge', max, 'credit'),
    event('two', 'huge', max, 'credit'),
    event('three', 'huge', 1, 'credit'),
  ));
  const output = join(directory, 'report.json');

  const result = runCli([input, output]);
  assert.equal(result.status, 0, result.stderr);

  const text = readFileSync(output, 'utf8');
  assert.match(text, /"balanceCents":18014398509481983\}/);
  assert.match(text, /"netCents":18014398509481983\}/);
});

test('invalid inputs fail with nonzero exit and preserve existing output', async (t) => {
  const invalidCases = {
    'malformed JSON': '{"id":"a","account":"x","cents":1,"kind":"credit"}\n{oops\n',
    'negative cents': jsonl(event('a', 'x', -1, 'credit')),
    'fractional cents': '{"id":"a","account":"x","cents":1.5,"kind":"credit"}\n',
    'string cents': '{"id":"a","account":"x","cents":"1","kind":"credit"}\n',
    'unsafe cents': '{"id":"a","account":"x","cents":9007199254740992,"kind":"credit"}\n',
    'empty id': jsonl(event('', 'x', 1, 'credit')),
    'empty account': jsonl(event('a', '', 1, 'credit')),
    'invalid kind': jsonl(event('a', 'x', 1, 'transfer')),
    'null record': 'null\n',
    'array record': '[]\n',
    'missing fields': '{"id":"a"}\n',
    'conflicting duplicate': jsonl(
      event('same', 'x', 1, 'credit'),
      event('same', 'x', 2, 'credit'),
    ),
  };

  for (const [name, content] of Object.entries(invalidCases)) {
    await t.test(name, () => {
      const { directory, input } = writeInput(content);
      const output = join(directory, 'report.json');
      writeFileSync(output, 'ORIGINAL CONTENT\n');

      const result = runCli([input, output]);
      assert.notEqual(result.status, 0, `expected failure for ${name}`);
      assert.match(result.stderr, /reconcile:/);
      assert.equal(readFileSync(output, 'utf8'), 'ORIGINAL CONTENT\n');
    });
  }
});

test('invalid UTF-8 fails without replacing output', () => {
  const directory = testDirectory();
  const input = join(directory, 'bad.jsonl');
  const output = join(directory, 'report.json');
  writeFileSync(input, Buffer.from([0x7b, 0x22, 0xff, 0x22, 0x7d]));
  writeFileSync(output, 'KEEP\n');

  const result = runCli([input, output]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /UTF-8/);
  assert.equal(readFileSync(output, 'utf8'), 'KEEP\n');
});

test('a missing input fails without replacing output', () => {
  const directory = testDirectory();
  const output = join(directory, 'report.json');
  writeFileSync(output, 'KEEP\n');

  const result = runCli([join(directory, 'does-not-exist.jsonl'), output]);
  assert.notEqual(result.status, 0);
  assert.equal(readFileSync(output, 'utf8'), 'KEEP\n');
});

test('a valid run replaces an existing output only after full validation', () => {
  const { directory, input } = writeInput(jsonl(
    event('a', 'alpha', 10, 'credit'),
  ));
  const output = join(directory, 'report.json');
  writeFileSync(output, 'OLD\n');

  const result = runCli([input, output]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(readFileSync(output, 'utf8')), {
    uniqueEvents: 1,
    duplicateEvents: 0,
    accounts: [{ account: 'alpha', balanceCents: 10 }],
    netCents: 10,
  });
});

test('CLI rejects bad usage and unknown options', () => {
  const noArgs = runCli([]);
  assert.notEqual(noArgs.status, 0);
  assert.match(noArgs.stderr, /usage:/i);

  const missingAccount = runCli(['in.jsonl', 'out.json', '--account']);
  assert.notEqual(missingAccount.status, 0);
  assert.match(missingAccount.stderr, /--account requires a value/);

  const unknown = runCli(['in.jsonl', 'out.json', '--bogus']);
  assert.notEqual(unknown.status, 0);
  assert.match(unknown.stderr, /Unknown option/);
});

test('CLI can safely use the input path as the output path', () => {
  const { input } = writeInput(jsonl(
    event('a', 'alpha', 15, 'credit'),
    event('b', 'alpha', 5, 'debit'),
  ));

  const result = runCli([input, input]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(readFileSync(input, 'utf8')), {
    uniqueEvents: 2,
    duplicateEvents: 0,
    accounts: [{ account: 'alpha', balanceCents: 10 }],
    netCents: 10,
  });
});
