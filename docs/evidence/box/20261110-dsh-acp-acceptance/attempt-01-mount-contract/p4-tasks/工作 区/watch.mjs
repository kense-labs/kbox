#!/usr/bin/env node
// Watch an inbox for atomically delivered ledger batches and write one report.

import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import {
  compareByCodePoint,
  formatReport,
  reduceLedger,
} from './ledger.mjs';
import { writeFileAtomic } from './reconcile.mjs';

const moduleDirectory = dirname(fileURLToPath(import.meta.url));

export const DEFAULT_INBOX = '/external/inbox';
export const DEFAULT_OUT_DIR = join(moduleDirectory, 'out');
export const START_MARKER = 'watch-started.json';
export const STREAM_REPORT = 'stream.json';
export const DONE_MARKER = 'DONE';

function sleep(milliseconds) {
  return new Promise((resolvePromise) => {
    setTimeout(resolvePromise, milliseconds);
  });
}

/** Batch files are named batch-NN.jsonl and delivered by atomic rename. */
export function isBatchFileName(name) {
  return name.startsWith('batch-') && name.endsWith('.jsonl');
}

function readUtf8File(filePath) {
  const bytes = readFileSync(filePath);
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    throw new Error(`Batch is not valid UTF-8: ${filePath}`);
  }
}

/** Serialize the stream array while preserving exact integer JSON values. */
export function formatStreamReport(entries) {
  const items = entries.map(({ batch, summary }) => (
    `{"batch":${JSON.stringify(batch)},"summary":${formatReport(summary).trim()}}`
  ));
  return `[${items.join(',')}]\n`;
}

/**
 * Run the watcher. Exported with path overrides so it can be tested without
 * touching the real inbox or the host-coordinated start marker.
 */
export async function runWatch(options = {}) {
  const inbox = options.inbox ?? DEFAULT_INBOX;
  const outDir = options.outDir ?? DEFAULT_OUT_DIR;
  const pollMs = options.pollMs ?? 250;

  mkdirSync(outDir, { recursive: true });

  // The host begins its 125-second delivery schedule only after seeing this
  // marker, so it must be durably present before the first inbox scan.
  writeFileAtomic(
    join(outDir, START_MARKER),
    `${JSON.stringify({ at: Date.now() })}\n`,
  );
  console.log(`watch: started marker written to ${join(outDir, START_MARKER)}`);

  const processed = new Map();

  function scanInbox() {
    const names = readdirSync(inbox, { withFileTypes: true })
      .filter((entry) => entry.isFile() && isBatchFileName(entry.name))
      .map((entry) => entry.name)
      .sort(compareByCodePoint);

    let newBatches = 0;
    for (const batch of names) {
      if (processed.has(batch)) continue;

      const summary = reduceLedger(readUtf8File(join(inbox, batch)));
      processed.set(batch, summary);
      newBatches += 1;
      console.log(
        `watch: processed ${batch} `
        + `(unique=${summary.uniqueEvents}, duplicates=${summary.duplicateEvents})`,
      );
    }
    return newBatches;
  }

  const donePath = join(inbox, DONE_MARKER);
  let done = false;

  while (!done) {
    const newBatches = scanInbox();

    if (existsSync(donePath)) {
      // DONE is written only after all batches, but scan once more after a
      // short pause so directory visibility can never leave a final batch out.
      scanInbox();
      await sleep(100);
      scanInbox();
      done = true;
      break;
    }

    if (newBatches === 0) {
      await sleep(pollMs);
    }
  }

  const entries = Array.from(processed.entries())
    .sort(([left], [right]) => compareByCodePoint(left, right))
    .map(([batch, summary]) => ({ batch, summary }));

  const streamPath = resolve(outDir, STREAM_REPORT);
  writeFileAtomic(streamPath, formatStreamReport(entries));
  console.log(`watch: DONE; wrote ${entries.length} batches to ${streamPath}`);
  return entries;
}

function isMainModule() {
  if (process.argv[1] === undefined) return false;
  try {
    return import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
  } catch {
    return false;
  }
}

if (isMainModule()) {
  runWatch().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`watch: ${message}`);
    process.exitCode = 1;
  });
}
