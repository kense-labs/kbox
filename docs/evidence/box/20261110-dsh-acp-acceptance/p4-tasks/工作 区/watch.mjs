#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { mkdir, readdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { TextDecoder } from "node:util";

import { LedgerError, reduceLedger } from "./ledger.mjs";

const DEFAULT_INBOX = "/external/inbox";
const DEFAULT_OUT_DIR = "/workspace/out";
const BATCH_PATTERN = /^batch-.+\.jsonl$/;

function isDoneMarker(name) {
  return name === "DONE" || name === "DONE.txt";
}

function decodeUtf8(buffer, label) {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
  } catch {
    throw new LedgerError(`${label} is not valid UTF-8`);
  }
}

async function writeAtomically(filePath, contents) {
  const resolved = path.resolve(filePath);
  const parent = path.dirname(resolved);
  await mkdir(parent, { recursive: true });
  const temporary = path.join(
    parent,
    `.${path.basename(resolved)}.${randomUUID()}.tmp`,
  );
  try {
    await writeFile(temporary, contents, { encoding: "utf8", flag: "wx" });
    await rename(temporary, resolved);
  } catch (error) {
    try {
      await rm(temporary, { force: true });
    } catch {
      // Best-effort cleanup; the destination is only replaced by rename.
    }
    throw error;
  }
}

export async function processBatch(inboxDir, name) {
  const filePath = path.join(inboxDir, name);
  const text = decodeUtf8(await readFile(filePath), name);
  return reduceLedger(text);
}

function compareBatchNames(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}

/**
 * Watch an inbox for batch-*.jsonl files and a DONE marker.
 *
 * The started file is written before the first directory scan because the
 * producer waits for it before beginning delivery.
 */
export async function runWatcher(options = {}) {
  const inboxDir = options.inboxDir ?? DEFAULT_INBOX;
  const outDir = options.outDir ?? DEFAULT_OUT_DIR;
  const startedPath = options.startedPath ?? path.join(outDir, "watch-started.json");
  const streamPath = options.streamPath ?? path.join(outDir, "stream.json");
  const pollMs = options.pollMs ?? 200;
  const settleMs = options.settleMs ?? 600;
  const maxDoneWaitMs = options.maxDoneWaitMs ?? 5_000;
  const log = options.log ?? (() => {});

  await mkdir(outDir, { recursive: true });
  await writeAtomically(startedPath, `${JSON.stringify({ at: Date.now() })}\n`);
  log(`started marker written to ${startedPath}`);

  const processed = new Set();
  const results = [];
  let doneSeenAt = null;
  let lastProgressAt = Date.now();

  const scan = async () => {
    const entries = await readdir(inboxDir, { withFileTypes: true });
    const discovered = [];

    for (const entry of entries) {
      if (!entry.isFile()) continue;
      if (BATCH_PATTERN.test(entry.name)) discovered.push(entry.name);
      if (isDoneMarker(entry.name)) {
        if (doneSeenAt === null) {
          doneSeenAt = Date.now();
          log(`DONE marker observed in ${inboxDir}`);
        }
      }
    }

    discovered.sort(compareBatchNames);
    let processedAny = false;
    for (const name of discovered) {
      if (processed.has(name)) continue;
      const summary = await processBatch(inboxDir, name);
      processed.add(name);
      results.push({ batch: name, summary });
      processedAny = true;
      lastProgressAt = Date.now();
      log(
        `processed ${name}: ${summary.uniqueEvents} unique, ` +
          `${summary.duplicateEvents} duplicate`,
      );
    }
    return processedAny;
  };

  try {
    while (true) {
      await scan();

      if (doneSeenAt !== null) {
        const quietFor = Date.now() - lastProgressAt;
        const waitedSinceDone = Date.now() - doneSeenAt;
        if (
          quietFor >= settleMs ||
          waitedSinceDone >= maxDoneWaitMs
        ) {
          break;
        }
      }

      await new Promise((resolve) => setTimeout(resolve, pollMs));
    }
  } catch (error) {
    if (error instanceof LedgerError) throw error;
    throw new LedgerError(`watch failed: ${error.message}`);
  }

  results.sort((a, b) => compareBatchNames(a.batch, b.batch));
  await writeAtomically(streamPath, `${JSON.stringify(results)}\n`);
  log(`wrote ${streamPath} with ${results.length} batches`);
  return results;
}

export async function main() {
  try {
    await runWatcher({
      log: (message) => process.stdout.write(`[watch] ${message}\n`),
    });
    return 0;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(`watch: ${message}\n`);
    return 1;
  }
}

const isDirectInvocation =
  typeof process.argv[1] === "string" &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;

if (isDirectInvocation) {
  const code = await main();
  process.exitCode = code;
}
