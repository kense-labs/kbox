#!/usr/bin/env node

/**
 * Streaming ledger watcher.
 *
 * The host starts delivering batch-NN.jsonl files only after it observes
 * /workspace/out/watch-started.json. Every batch is processed exactly once,
 * in filename order, and the combined summaries are written to stream.json
 * after the DONE marker appears.
 */

import { randomUUID } from "node:crypto";
import {
  mkdir,
  readFile,
  readdir,
  rename,
  unlink,
  writeFile,
} from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { compareByCodePoint, parseJsonl, reduceEvents } from "./ledger.mjs";

const DEFAULT_INBOX = "/external/inbox";
const DEFAULT_OUT_DIR = "/workspace/out";
const DEFAULT_POLL_MS = 250;

const BATCH_PATTERN = /^batch-.*\.jsonl$/i;

function delay(milliseconds) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
}

async function readUtf8File(path) {
  const buffer = await readFile(path);
  return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
}

async function writeAtomic(outputPath, contents) {
  const directory = dirname(outputPath);
  await mkdir(directory, { recursive: true });

  const temporaryPath = join(
    directory,
    `.${basename(outputPath)}.${process.pid}.${randomUUID()}.tmp`,
  );

  try {
    await writeFile(temporaryPath, contents, { encoding: "utf8", flag: "wx" });
    await rename(temporaryPath, outputPath);
  } catch (error) {
    try {
      await unlink(temporaryPath);
    } catch {
      // The temporary file may never have been created.
    }
    throw error;
  }
}

function isDoneMarker(name) {
  const upper = name.toUpperCase();
  return upper === "DONE" || upper.startsWith("DONE.");
}

function batchNames(entries) {
  return entries
    .filter((entry) => BATCH_PATTERN.test(entry.name))
    .map((entry) => entry.name)
    .sort(compareByCodePoint);
}

async function listInbox(inbox) {
  try {
    const entries = await readdir(inbox, { withFileTypes: true });
    return {
      batches: batchNames(entries),
      done: entries.some((entry) => isDoneMarker(entry.name)),
    };
  } catch (error) {
    if (error && error.code === "ENOENT") {
      return { batches: [], done: false };
    }
    throw error;
  }
}

/**
 * Watch the inbox until DONE, then write the sorted stream summary.
 * Exported so the behavior can be exercised with a synthetic inbox.
 */
export async function runWatch(options = {}) {
  const inbox = options.inbox ?? DEFAULT_INBOX;
  const outDir = options.outDir ?? DEFAULT_OUT_DIR;
  const pollMs = options.pollMs ?? DEFAULT_POLL_MS;
  const startedPath = join(outDir, "watch-started.json");
  const streamPath = join(outDir, "stream.json");

  // The host waits for this marker, so write it before scanning the inbox.
  await writeAtomic(startedPath, `${JSON.stringify({ at: Date.now() })}\n`);

  const seen = new Set();
  const summaries = new Map();

  async function processNames(names) {
    for (const name of names) {
      if (seen.has(name)) {
        continue;
      }

      let text;
      try {
        text = await readUtf8File(join(inbox, name));
      } catch (error) {
        // An atomic rename can make a name disappear between readdir and read.
        if (error && error.code === "ENOENT") {
          continue;
        }
        throw error;
      }

      const summary = reduceEvents(parseJsonl(text));
      seen.add(name);
      summaries.set(name, summary);
    }
  }

  while (true) {
    const snapshot = await listInbox(inbox);
    await processNames(snapshot.batches);

    if (!snapshot.done) {
      await delay(pollMs);
      continue;
    }

    // DONE means the host has finished delivering. Do a final sweep so any
    // batch that became visible together with DONE is still included.
    const finalSnapshot = await listInbox(inbox);
    await processNames(finalSnapshot.batches);

    const remaining = finalSnapshot.batches.filter((name) => !seen.has(name));
    if (remaining.length > 0) {
      continue;
    }
    break;
  }

  const results = Array.from(summaries, ([batch, summary]) => ({
    batch,
    summary,
  })).sort((a, b) => compareByCodePoint(a.batch, b.batch));

  await writeAtomic(streamPath, `${JSON.stringify(results, null, 2)}\n`);
  return results;
}

function isDirectRun() {
  const invokedPath = process.argv[1];
  return (
    typeof invokedPath === "string" &&
    resolve(invokedPath) === fileURLToPath(import.meta.url)
  );
}

if (isDirectRun()) {
  runWatch().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(`watch: ${message}\n`);
    process.exitCode = 1;
  });
}
