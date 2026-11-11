import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  compareCodePoints,
  LedgerError,
  parseJsonl,
  reduceEvents,
  reduceLedger,
} from "./ledger.mjs";

const CLI = fileURLToPath(new URL("./reconcile.mjs", import.meta.url));

function jsonl(records) {
  return `${records.map((record) => JSON.stringify(record)).join("\n")}\n`;
}

function runCli(args, options = {}) {
  return spawnSync(process.execPath, [CLI, ...args], {
    encoding: "utf8",
    ...options,
  });
}

async function inTempDir(fn) {
  const directory = await mkdtemp(path.join(os.tmpdir(), "ledger-test-"));
  try {
    return await fn(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

test("reducer adds credits, subtracts debits, and sorts accounts by code point", () => {
  const text = jsonl([
    { id: "a", account: "alpha", cents: 1000, kind: "credit" },
    { id: "b", account: "alpha", cents: 250, kind: "debit" },
    { id: "c", account: "😀", cents: 7, kind: "credit" },
    { id: "d", account: "A", cents: 3, kind: "credit" },
    { id: "e", account: "é", cents: 11, kind: "credit" },
    { id: "f", account: "\uFFFF", cents: 13, kind: "credit" },
  ]);

  assert.deepEqual(reduceLedger(text), {
    uniqueEvents: 6,
    duplicateEvents: 0,
    accounts: [
      { account: "A", balanceCents: 3 },
      { account: "alpha", balanceCents: 750 },
      { account: "é", balanceCents: 11 },
      { account: "\uFFFF", balanceCents: 13 },
      { account: "😀", balanceCents: 7 },
    ],
    netCents: 784,
  });
});

test("identical duplicate ids are ignored and counted; blanks are ignored", () => {
  const first = { id: "same", account: "a", cents: 42, kind: "credit" };
  const another = { id: "other", account: "a", cents: 8, kind: "debit" };
  const text = `\n${JSON.stringify(first)}\n   \n${JSON.stringify(another)}\n${JSON.stringify(first)}\n${JSON.stringify(first)}\n\n`;

  assert.deepEqual(reduceLedger(text), {
    uniqueEvents: 2,
    duplicateEvents: 2,
    accounts: [{ account: "a", balanceCents: 34 }],
    netCents: 34,
  });
});

test("conflicting records with the same id invalidate the whole input", () => {
  const text = jsonl([
    { id: "dup", account: "a", cents: 10, kind: "credit" },
    { id: "dup", account: "a", cents: 11, kind: "credit" },
  ]);

  assert.throws(() => reduceLedger(text), (error) => {
    assert.ok(error instanceof LedgerError);
    assert.match(error.message, /conflicting records share id/);
    return true;
  });
});

test("integer arithmetic stays exact for large safe integers", () => {
  const text = jsonl([
    { id: "one", account: "big", cents: 2_000_000_000, kind: "credit" },
    { id: "two", account: "big", cents: 1_337_000_001, kind: "credit" },
    { id: "three", account: "big", cents: 999_999_999, kind: "debit" },
  ]);

  const result = reduceLedger(text);
  assert.equal(result.accounts[0].balanceCents, 2_337_000_002);
  assert.equal(result.netCents, 2_337_000_002);
  assert.ok(Number.isSafeInteger(result.netCents));

  assert.throws(
    () =>
      reduceEvents([
        { id: "max", account: "x", cents: Number.MAX_SAFE_INTEGER, kind: "credit" },
        { id: "over", account: "x", cents: 1, kind: "credit" },
      ]),
    /overflow/,
  );
});

test("malformed JSON and invalid fields are rejected", () => {
  assert.throws(() => parseJsonl('{"id":"x",\n'), /not valid JSON/);
  assert.throws(() => parseJsonl("[]\n"), /must be a JSON object/);
  assert.throws(() => parseJsonl("null\n"), /must be a JSON object/);

  const valid = { id: "id", account: "acct", cents: 1, kind: "credit" };
  const invalidRecords = [
    { ...valid, id: "" },
    { ...valid, id: 7 },
    { ...valid, account: "" },
    { ...valid, account: null },
    { ...valid, cents: -1 },
    { ...valid, cents: 1.5 },
    { ...valid, cents: "1" },
    { ...valid, cents: Number.MAX_SAFE_INTEGER + 1 },
    { ...valid, kind: "transfer" },
    { ...valid, kind: null },
  ];

  for (const record of invalidRecords) {
    assert.throws(() => reduceEvents([record]), LedgerError, JSON.stringify(record));
  }

  const missingFields = { id: "missing" };
  assert.throws(() => reduceEvents([missingFields]), /invalid account/);
});

test("--account filtering keeps global event counts and filters netCents", () => {
  const text = jsonl([
    { id: "1", account: "alpha", cents: 100, kind: "credit" },
    { id: "2", account: "beta", cents: 40, kind: "credit" },
    { id: "3", account: "beta", cents: 15, kind: "debit" },
    { id: "2", account: "beta", cents: 40, kind: "credit" },
  ]);

  assert.deepEqual(reduceLedger(text, { account: "beta" }), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [{ account: "beta", balanceCents: 25 }],
    netCents: 25,
  });

  assert.deepEqual(reduceLedger(text, { account: "missing" }), {
    uniqueEvents: 3,
    duplicateEvents: 1,
    accounts: [],
    netCents: 0,
  });
});

test("compareCodePoints orders astral characters after BMP code points", () => {
  const values = ["😀", "\uFFFF", "a", "A", "é"];
  assert.deepEqual(values.sort(compareCodePoints), ["A", "a", "é", "\uFFFF", "😀"]);
});

test("CLI writes the exact summary, creates parent directories, and filters", async () => {
  await inTempDir(async (directory) => {
    const input = path.join(directory, "events.jsonl");
    const output = path.join(directory, "nested", "deep", "汇总 report.json");
    await writeFile(
      input,
      jsonl([
        { id: "1", account: "alpha", cents: 500, kind: "credit" },
        { id: "2", account: "beta", cents: 125, kind: "debit" },
        { id: "1", account: "alpha", cents: 500, kind: "credit" },
      ]),
      "utf8",
    );

    const run = runCli([input, output, "--account", "beta"]);
    assert.equal(run.status, 0, run.stderr);
    assert.equal(run.stdout, "");

    const written = JSON.parse(await readFile(output, "utf8"));
    assert.deepEqual(Object.keys(written), [
      "uniqueEvents",
      "duplicateEvents",
      "accounts",
      "netCents",
    ]);
    assert.deepEqual(written, {
      uniqueEvents: 2,
      duplicateEvents: 1,
      accounts: [{ account: "beta", balanceCents: -125 }],
      netCents: -125,
    });
  });
});

test("CLI leaves an existing output untouched on malformed input", async () => {
  await inTempDir(async (directory) => {
    const input = path.join(directory, "bad.jsonl");
    const output = path.join(directory, "report.json");
    const sentinel = '{"keep":"me"}\n';
    await writeFile(output, sentinel, "utf8");
    await writeFile(input, '{"id":"x","account":"a","cents":1,"kind":"credit"}\nnot json\n', "utf8");

    const run = runCli([input, output]);
    assert.notEqual(run.status, 0);
    assert.match(run.stderr, /not valid JSON/);
    assert.equal(await readFile(output, "utf8"), sentinel);
  });
});

test("CLI leaves an existing output untouched on conflicting duplicate ids", async () => {
  await inTempDir(async (directory) => {
    const input = path.join(directory, "conflict.jsonl");
    const output = path.join(directory, "report.json");
    const sentinel = "unchanged";
    await writeFile(output, sentinel, "utf8");
    await writeFile(
      input,
      jsonl([
        { id: "same", account: "a", cents: 1, kind: "credit" },
        { id: "same", account: "a", cents: 2, kind: "credit" },
      ]),
      "utf8",
    );

    const run = runCli([input, output]);
    assert.notEqual(run.status, 0);
    assert.match(run.stderr, /conflicting records/);
    assert.equal(await readFile(output, "utf8"), sentinel);
  });
});

test("CLI rejects invalid UTF-8, missing inputs, bad arguments, and bad fields", async () => {
  await inTempDir(async (directory) => {
    const output = path.join(directory, "report.json");
    const sentinel = "preserve";
    await writeFile(output, sentinel, "utf8");

    const invalidUtf8 = path.join(directory, "invalid-utf8.jsonl");
    await writeFile(invalidUtf8, Buffer.from([0xff, 0xfe, 0x7b, 0x7d]));
    let run = runCli([invalidUtf8, output]);
    assert.notEqual(run.status, 0);
    assert.match(run.stderr, /valid UTF-8/);
    assert.equal(await readFile(output, "utf8"), sentinel);

    run = runCli([path.join(directory, "absent.jsonl"), output]);
    assert.notEqual(run.status, 0);
    assert.equal(await readFile(output, "utf8"), sentinel);

    const badField = path.join(directory, "bad-field.jsonl");
    await writeFile(
      badField,
      jsonl([{ id: "x", account: "a", cents: -5, kind: "credit" }]),
      "utf8",
    );
    run = runCli([badField, output]);
    assert.notEqual(run.status, 0);
    assert.match(run.stderr, /invalid cents/);
    assert.equal(await readFile(output, "utf8"), sentinel);

    run = runCli([badField, output, "--bogus"]);
    assert.notEqual(run.status, 0);
    assert.match(run.stderr, /unknown option/);
    assert.equal(await readFile(output, "utf8"), sentinel);

    run = runCli([badField]);
    assert.notEqual(run.status, 0);
    assert.match(run.stderr, /usage:/);
    assert.equal(await readFile(output, "utf8"), sentinel);
  });
});

test("empty input yields an empty but well-formed summary", () => {
  assert.deepEqual(reduceLedger("\n \n\t\n"), {
    uniqueEvents: 0,
    duplicateEvents: 0,
    accounts: [],
    netCents: 0,
  });
});

test("CLI replaces a previous output after a successful run", async () => {
  await inTempDir(async (directory) => {
    const input = path.join(directory, "events.jsonl");
    const output = path.join(directory, "report.json");
    await mkdir(path.dirname(output), { recursive: true });
    await writeFile(output, "old data", "utf8");
    await writeFile(
      input,
      jsonl([{ id: "1", account: "a", cents: 9, kind: "credit" }]),
      "utf8",
    );

    const run = runCli([input, output]);
    assert.equal(run.status, 0, run.stderr);
    const parsed = JSON.parse(await readFile(output, "utf8"));
    assert.equal(parsed.uniqueEvents, 1);
    assert.equal(parsed.accounts[0].balanceCents, 9);
  });
});
