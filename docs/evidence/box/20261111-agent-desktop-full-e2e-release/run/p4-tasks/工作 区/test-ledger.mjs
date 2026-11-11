import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  LedgerError,
  compareByCodePoint,
  parseJsonl,
  reduceEvents,
} from "./ledger.mjs";

const CLI = fileURLToPath(new URL("./reconcile.mjs", import.meta.url));

function runCli(args) {
  return spawnSync(process.execPath, [CLI, ...args], {
    encoding: "utf8",
  });
}

async function makeTempDir(t) {
  const directory = await mkdtemp(join(tmpdir(), "ledger-test-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}

function event(id, account, cents, kind) {
  return { id, account, cents, kind };
}

test("identical duplicate ids are counted once and reported", () => {
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

test("conflicting records with the same id invalidate the input", () => {
  assert.throws(
    () =>
      reduceEvents([
        event("same", "alpha", 100, "credit"),
        event("same", "alpha", 101, "credit"),
      ]),
    (error) =>
      error instanceof LedgerError &&
      /conflicting records share id "same"/.test(error.message),
  );

  assert.throws(
    () =>
      reduceEvents([
        event("same", "alpha", 100, "credit"),
        event("same", "alpha", 100, "debit"),
      ]),
    LedgerError,
  );
});

test("integer arithmetic stays exact at the safe integer boundary", () => {
  const result = reduceEvents([
    event("max", "big", Number.MAX_SAFE_INTEGER, "credit"),
    event("minus", "big", Number.MAX_SAFE_INTEGER - 1, "debit"),
    event("small", "small", 1, "credit"),
  ]);

  assert.deepEqual(result.accounts, [
    { account: "big", balanceCents: 1 },
    { account: "small", balanceCents: 1 },
  ]);
  assert.equal(result.netCents, 2);

  assert.throws(
    () =>
      reduceEvents([
        event("max", "big", Number.MAX_SAFE_INTEGER, "credit"),
        event("overflow", "big", 1, "credit"),
      ]),
    /safe integer range/,
  );
});

test("invalid fields are rejected", () => {
  const invalidEvents = [
    null,
    [],
    {},
    { id: "", account: "alpha", cents: 1, kind: "credit" },
    { id: 7, account: "alpha", cents: 1, kind: "credit" },
    { id: "a", account: "", cents: 1, kind: "credit" },
    { id: "a", account: "alpha", cents: -1, kind: "credit" },
    { id: "a", account: "alpha", cents: 1.5, kind: "credit" },
    { id: "a", account: "alpha", cents: Number.MAX_SAFE_INTEGER + 1, kind: "credit" },
    { id: "a", account: "alpha", cents: 1, kind: "Credit" },
    { id: "a", account: "alpha", cents: 1, kind: "credit", extra: true },
  ];

  for (const invalid of invalidEvents) {
    assert.throws(() => reduceEvents([invalid]), LedgerError);
  }
});

test("accounts sort by Unicode code point, including astral characters", () => {
  const names = ["a", "Z", "é", "\uFFFF", "😀"];
  const result = reduceEvents(
    names.map((account, index) => event(`id-${index}`, account, 1, "credit")),
  );

  assert.deepEqual(
    result.accounts.map((entry) => entry.account),
    ["Z", "a", "é", "\uFFFF", "😀"],
  );
  assert.equal(compareByCodePoint("😀", "\uFFFF"), 1);
});

test("parseJsonl ignores blank lines and reports malformed line numbers", () => {
  const events = parseJsonl(
    '\n{"id":"a","account":"alpha","cents":1,"kind":"credit"}\r\n   \n',
  );
  assert.equal(events.length, 1);

  assert.throws(() => parseJsonl("\n\nnot json\n"), /line 3: malformed JSON/);
});

test("CLI writes the exact shape, creates parent directories and deduplicates", async (t) => {
  const directory = await makeTempDir(t);
  const input = join(directory, "input.jsonl");
  const output = join(directory, "nested", "deep", "report.json");

  await writeFile(
    input,
    [
      '{"id":"a","account":"alpha","cents":500,"kind":"credit"}',
      "",
      '{"id":"b","account":"beta","cents":125,"kind":"debit"}',
      '{"id":"a","account":"alpha","cents":500,"kind":"credit"}',
      "   ",
      '{"id":"c","account":"alpha","cents":25,"kind":"credit"}',
      "",
    ].join("\n"),
    "utf8",
  );

  const run = runCli([input, output]);
  assert.equal(run.status, 0, run.stderr);

  const parsed = JSON.parse(await readFile(output, "utf8"));
  assert.deepEqual(Object.keys(parsed), [
    "uniqueEvents",
    "duplicateEvents",
    "accounts",
    "netCents",
  ]);
  assert.deepEqual(parsed, {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [
      { account: "alpha", balanceCents: 525 },
      { account: "beta", balanceCents: -125 },
    ],
    netCents: 400,
  });
});

test("--account filters balances but not whole-input event counts", async (t) => {
  const directory = await makeTempDir(t);
  const input = join(directory, "input.jsonl");
  const output = join(directory, "filtered.json");

  await writeFile(
    input,
    [
      '{"id":"a","account":"alpha","cents":50,"kind":"credit"}',
      '{"id":"b","account":"beta","cents":20,"kind":"debit"}',
      '{"id":"a","account":"alpha","cents":50,"kind":"credit"}',
      '{"id":"c","account":"alpha","cents":25,"kind":"credit"}',
    ].join("\n"),
    "utf8",
  );

  let run = runCli([input, output, "--account", "alpha"]);
  assert.equal(run.status, 0, run.stderr);
  assert.deepEqual(JSON.parse(await readFile(output, "utf8")), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: "alpha", balanceCents: 75 }],
    netCents: 75,
  });

  run = runCli(["--account=missing", input, output]);
  assert.equal(run.status, 0, run.stderr);
  assert.deepEqual(JSON.parse(await readFile(output, "utf8")), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [],
    netCents: 0,
  });
});

test("invalid input fails nonzero and preserves an existing output", async (t) => {
  const directory = await makeTempDir(t);
  const input = join(directory, "input.jsonl");
  const output = join(directory, "existing.json");
  const sentinel = '{"keep":"this exact content"}\n';

  await writeFile(output, sentinel, "utf8");

  const badInputs = [
    '{"id":"a","account":"alpha","cents":1,"kind":"credit"}\nnot json\n',
    '{"id":"a","account":"alpha","cents":-1,"kind":"credit"}\n',
    '{"id":"a","account":"alpha","cents":1,"kind":"credit"}\n' +
      '{"id":"a","account":"alpha","cents":2,"kind":"credit"}\n',
    '{"id":"a","account":"alpha","cents":1,"kind":"credit","extra":1}\n',
  ];

  for (const contents of badInputs) {
    await writeFile(input, contents, "utf8");
    const run = runCli([input, output]);
    assert.equal(run.status, 1, `expected failure for: ${contents}`);
    assert.notEqual(run.stderr.length, 0);
    assert.equal(await readFile(output, "utf8"), sentinel);
  }

  // A missing input is also an error before the output is replaced.
  const run = runCli([join(directory, "does-not-exist.jsonl"), output]);
  assert.equal(run.status, 1);
  assert.equal(await readFile(output, "utf8"), sentinel);
});

test("invalid UTF-8 fails without replacing an existing output", async (t) => {
  const directory = await makeTempDir(t);
  const input = join(directory, "invalid.jsonl");
  const output = join(directory, "existing.json");

  await writeFile(input, Buffer.from([0x7b, 0xff, 0x7d]));
  await writeFile(output, "preserve me", "utf8");

  const run = runCli([input, output]);
  assert.equal(run.status, 1);
  assert.match(run.stderr, /not valid UTF-8/);
  assert.equal(await readFile(output, "utf8"), "preserve me");
});
