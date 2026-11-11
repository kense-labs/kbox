import test from "node:test";
import assert from "node:assert/strict";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  LedgerError,
  compareByCodePoint,
  parseJsonl,
  reduceEvents,
} from "./ledger.mjs";

const CLI = fileURLToPath(new URL("./reconcile.mjs", import.meta.url));

function makeTempDir(t) {
  const directory = mkdtempSync(join(tmpdir(), "ledger-test-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  return directory;
}

function runCli(args, cwd) {
  return spawnSync(process.execPath, [CLI, ...args], {
    encoding: "utf8",
    cwd,
  });
}

function event(id, account, cents, kind) {
  return { id, account, cents, kind };
}

test("reducer totals credits and debits and deduplicates identical ids", () => {
  const result = reduceEvents([
    event("a", "alpha", 100, "credit"),
    event("b", "beta", 40, "debit"),
    event("a", "alpha", 100, "credit"),
    event("a", "alpha", 100, "credit"),
  ]);

  assert.deepEqual(result, {
    uniqueEvents: 2,
    duplicateEvents: 2,
    accounts: [
      { account: "alpha", balanceCents: 100 },
      { account: "beta", balanceCents: -40 },
    ],
    netCents: 60,
  });
});

test("conflicting records sharing an id invalidate the whole input", () => {
  assert.throws(
    () =>
      reduceEvents([
        event("same", "alpha", 10, "credit"),
        event("same", "alpha", 11, "credit"),
      ]),
    (error) => error instanceof LedgerError && /conflicting/.test(error.message)
  );

  assert.throws(
    () =>
      reduceEvents([
        event("same", "alpha", 10, "credit"),
        event("same", "alpha", 10, "debit"),
      ]),
    LedgerError
  );
});

test("integer arithmetic stays exact near the safe-integer boundary", () => {
  const result = reduceEvents([
    event("1", "big", Number.MAX_SAFE_INTEGER, "credit"),
    event("2", "big", Number.MAX_SAFE_INTEGER - 1, "debit"),
    event("3", "big", 1, "credit"),
  ]);

  assert.deepEqual(result.accounts, [{ account: "big", balanceCents: 2 }]);
  assert.equal(result.netCents, 2);

  const exact = reduceEvents([
    event("1", "big", 9007199254740990, "credit"),
    event("2", "big", 1, "debit"),
  ]);
  assert.equal(exact.netCents, 9007199254740989);
  assert.equal(exact.accounts[0].balanceCents, 9007199254740989);

  // Zero-cent events and duplicate zero-cent events are legal.
  assert.deepEqual(
    reduceEvents([
      event("z", "zero", 0, "credit"),
      event("z", "zero", 0, "credit"),
    ]),
    {
      uniqueEvents: 1,
      duplicateEvents: 1,
      accounts: [{ account: "zero", balanceCents: 0 }],
      netCents: 0,
    }
  );
});

test("invalid fields and malformed JSON are rejected", () => {
  const invalidEvents = [
    [null],
    [[]],
    [event("", "a", 1, "credit")],
    [event("  ", "a", 1, "credit")],
    [event("id", "", 1, "credit")],
    [event("id", "  ", 1, "credit")],
    [event("id", "a", -1, "credit")],
    [event("id", "a", 1.5, "credit")],
    [event("id", "a", Number.MAX_SAFE_INTEGER + 1, "credit")],
    [event("id", "a", "1", "credit")],
    [event("id", "a", 1, "transfer")],
    [event("id", "a", 1)],
    [{ id: "id", account: "a", kind: "credit" }],
  ];

  for (const events of invalidEvents) {
    assert.throws(
      () => reduceEvents(events),
      LedgerError,
      `expected rejection for ${JSON.stringify(events)}`
    );
  }

  assert.throws(() => parseJsonl('{"id":"a",}\n'), /malformed JSON/);
  assert.throws(() => parseJsonl("[]\n"), LedgerError);
  assert.throws(() => parseJsonl("null\n"), LedgerError);
});

test("--account filtering changes accounts and net but not event counts", () => {
  const events = [
    event("1", "alpha", 10, "credit"),
    event("2", "beta", 20, "credit"),
    event("3", "alpha", 4, "debit"),
    event("2", "beta", 20, "credit"),
  ];

  assert.deepEqual(reduceEvents(events, { account: "alpha" }), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: "alpha", balanceCents: 6 }],
    netCents: 6,
  });

  // The filter also accepts the shorthand string argument.
  assert.deepEqual(reduceEvents(events, "beta"), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: "beta", balanceCents: 20 }],
    netCents: 20,
  });

  assert.deepEqual(reduceEvents(events, { account: "missing" }), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [],
    netCents: 0,
  });
});

test("--account filtering supports Unicode account names", () => {
  const events = [
    event("1", "账户甲", 10, "credit"),
    event("2", "账户乙", 20, "credit"),
    event("3", "账户甲", 4, "debit"),
    event("1", "账户甲", 10, "credit"),
  ];

  assert.deepEqual(reduceEvents(events, { account: "账户甲" }), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: "账户甲", balanceCents: 6 }],
    netCents: 6,
  });

  assert.deepEqual(reduceEvents(events), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [
      { account: "账户乙", balanceCents: 20 },
      { account: "账户甲", balanceCents: 6 },
    ],
    netCents: 26,
  });
});

test("accounts are sorted by Unicode code point", () => {
  const names = ["\u{10000}", "\uE000", "ä", "a", "Z", "账户甲", "账户乙"];
  const result = reduceEvents(names.map((name, i) => event(String(i), name, 1, "credit")));

  assert.deepEqual(
    result.accounts.map((entry) => entry.account),
    ["Z", "a", "ä", "账户乙", "账户甲", "\uE000", "\u{10000}"]
  );

  // UTF-16 code-unit sorting would put the astral character before U+E000.
  assert.ok(compareByCodePoint("\uE000", "\u{10000}") < 0);
  assert.ok(compareByCodePoint("\u{10000}", "\uE000") > 0);
});

test("blank lines are ignored when parsing JSONL", () => {
  const text =
    '\r\n{"id":"a","account":"x","cents":1,"kind":"credit"}\n   \n\t\n' +
    '{"id":"a","account":"x","cents":1,"kind":"credit"}\r\n';
  const events = parseJsonl(text);
  assert.equal(events.length, 2);

  assert.deepEqual(reduceEvents(text), {
    uniqueEvents: 1,
    duplicateEvents: 1,
    accounts: [{ account: "x", balanceCents: 1 }],
    netCents: 1,
  });
});

test("CLI writes the expected JSON and creates parent directories", (t) => {
  const dir = makeTempDir(t);
  const input = join(dir, "input.jsonl");
  const output = join(dir, "nested", "deeper", "result.json");
  writeFileSync(
    input,
    [
      '{"id":"1","account":"alpha","cents":15,"kind":"credit"}',
      "",
      '{"id":"2","account":"beta","cents":5,"kind":"debit"}',
      '{"id":"1","account":"alpha","cents":15,"kind":"credit"}',
      "",
    ].join("\n")
  );

  const run = runCli([input, output], dir);
  assert.equal(run.status, 0, run.stderr);
  assert.equal(run.stderr, "");

  const parsed = JSON.parse(readFileSync(output, "utf8"));
  assert.deepEqual(parsed, {
    uniqueEvents: 2,
    duplicateEvents: 1,
    accounts: [
      { account: "alpha", balanceCents: 15 },
      { account: "beta", balanceCents: -5 },
    ],
    netCents: 10,
  });
});

test("CLI --account filters output while keeping whole-input counts", (t) => {
  const dir = makeTempDir(t);
  const input = join(dir, "input.jsonl");
  const output = join(dir, "result.json");
  writeFileSync(
    input,
    [
      '{"id":"1","account":"alpha","cents":15,"kind":"credit"}',
      '{"id":"2","account":"beta","cents":5,"kind":"debit"}',
      '{"id":"2","account":"beta","cents":5,"kind":"debit"}',
    ].join("\n")
  );

  const run = runCli([input, output, "--account", "beta"], dir);
  assert.equal(run.status, 0, run.stderr);
  assert.deepEqual(JSON.parse(readFileSync(output, "utf8")), {
    uniqueEvents: 2,
    duplicateEvents: 1,
    accounts: [{ account: "beta", balanceCents: -5 }],
    netCents: -5,
  });
});

test("CLI preserves an existing output when input is malformed", (t) => {
  const dir = makeTempDir(t);
  const input = join(dir, "bad.jsonl");
  const output = join(dir, "result.json");
  writeFileSync(input, '{"id":"1","account":"a","cents":1,"kind":"credit"}\nnot json\n');
  writeFileSync(output, "EXISTING OUTPUT\n");

  const run = runCli([input, output], dir);
  assert.notEqual(run.status, 0);
  assert.match(run.stderr, /error:/);
  assert.equal(readFileSync(output, "utf8"), "EXISTING OUTPUT\n");
});

test("CLI preserves an existing output on invalid or conflicting events", (t) => {
  const dir = makeTempDir(t);
  const output = join(dir, "result.json");

  const cases = [
    '{"id":"1","account":"a","cents":-2,"kind":"credit"}\n',
    '{"id":"1","account":"a","cents":2,"kind":"credit"}\n' +
      '{"id":"1","account":"a","cents":3,"kind":"credit"}\n',
    '{"id":"1","account":"a","cents":2,"kind":"other"}\n',
  ];

  for (let i = 0; i < cases.length; i += 1) {
    const input = join(dir, `invalid-${i}.jsonl`);
    writeFileSync(input, cases[i]);
    writeFileSync(output, `KEEP ${i}\n`);

    const run = runCli([input, output], dir);
    assert.notEqual(run.status, 0);
    assert.equal(readFileSync(output, "utf8"), `KEEP ${i}\n`);
  }
});

test("CLI rejects missing arguments and unknown options without writing output", (t) => {
  const dir = makeTempDir(t);
  const output = join(dir, "result.json");

  const missingOutput = runCli([join(dir, "no-such-input.jsonl")], dir);
  assert.notEqual(missingOutput.status, 0);
  assert.match(missingOutput.stderr, /usage|required/i);

  const unknownOption = runCli([join(dir, "no-such-input.jsonl"), output, "--bogus"], dir);
  assert.notEqual(unknownOption.status, 0);
  assert.match(unknownOption.stderr, /unknown option/);

  const noInput = runCli([join(dir, "missing.jsonl"), output], dir);
  assert.notEqual(noInput.status, 0);
  assert.match(noInput.stderr, /cannot read input/);
});
