#!/usr/bin/env node
/**
 * Watch a directory for atomic JSONL batch files and reduce them once each.
 *
 * Batches are named batch-NN.jsonl.  A file named DONE signals that no more
 * batches will arrive; after every discovered batch has been processed the
 * summaries are written (sorted by batch filename) to the output JSON file.
 *
 * Usage: node watch.mjs [INBOX_DIR] [OUTPUT_FILE]
 */

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { parseJsonl, reduceEvents } from "./ledger.mjs";
import { writeOutputAtomically } from "./reconcile.mjs";

const DEFAULT_INBOX = "/external/inbox";
const DEFAULT_OUTPUT = "/workspace/out/stream.json";
const BATCH_PATTERN = /^batch-\d+\.jsonl$/;
const POLL_MS = 250;

const inboxDir = process.argv[2] ?? DEFAULT_INBOX;
const outputPath = process.argv[3] ?? DEFAULT_OUTPUT;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function listBatchFiles() {
  let entries;
  try {
    entries = readdirSync(inboxDir, { withFileTypes: true });
  } catch (error) {
    if (error.code === "ENOENT") return [];
    throw error;
  }

  return entries
    .filter((entry) => entry.isFile() && BATCH_PATTERN.test(entry.name))
    .map((entry) => entry.name)
    .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

function doneExists() {
  return existsSync(join(inboxDir, "DONE"));
}

function processBatch(name) {
  const text = readFileSync(join(inboxDir, name), "utf8");
  const summary = reduceEvents(parseJsonl(text));
  return { batch: name, summary };
}

async function main() {
  const processed = new Set();
  const results = new Map();

  for (;;) {
    const done = doneExists();
    const pending = listBatchFiles().filter((name) => !processed.has(name));

    for (const name of pending) {
      const result = processBatch(name);
      results.set(name, result);
      processed.add(name);
      console.log(
        `processed ${name}: ${result.summary.uniqueEvents} unique, ` +
          `${result.summary.duplicateEvents} duplicate`
      );
    }

    if (done) {
      // Give a final atomic batch rename a moment to become visible before
      // deciding that all work is complete.
      await sleep(POLL_MS);
      const late = listBatchFiles().filter((name) => !processed.has(name));
      if (late.length === 0) {
        const output = [...results.values()].sort((a, b) =>
          a.batch < b.batch ? -1 : a.batch > b.batch ? 1 : 0
        );
        writeOutputAtomically(
          outputPath,
          `${JSON.stringify(output, null, 2)}\n`
        );
        console.log(`DONE: wrote ${output.length} batches to ${outputPath}`);
        return;
      }
      continue;
    }

    await sleep(POLL_MS);
  }
}

main().catch((error) => {
  process.stderr.write(`error: ${error instanceof Error ? error.message : error}\n`);
  process.exitCode = 1;
});
