#!/usr/bin/env node
// Streaming watcher: node watch.mjs
//
// Writes out/watch-started.json first, then polls /external/inbox for
// batch-*.jsonl files (each processed exactly once, in filename order) and
// stops once the DONE marker is present and every batch has been processed.
// The final stream is written atomically to out/stream.json.

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { compareByCodePoint, parseLedgerText, reduceLedger } from './ledger.mjs';

const workspace = path.dirname(fileURLToPath(import.meta.url));
const INBOX = '/external/inbox';
const OUT_DIR = path.join(workspace, 'out');
const STARTED = path.join(OUT_DIR, 'watch-started.json');
const STREAM = path.join(OUT_DIR, 'stream.json');
const BATCH_PATTERN = /^batch-.+\.jsonl$/;
const POLL_MS = 500;
const MAX_WAIT_MS = 600_000;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function writeAtomic(file, contents) {
  const directory = path.dirname(file);
  fs.mkdirSync(directory, { recursive: true });
  const temporary = path.join(
    directory,
    `.${path.basename(file)}.${process.pid}.${crypto.randomBytes(6).toString('hex')}.tmp`,
  );
  try {
    fs.writeFileSync(temporary, contents, { encoding: 'utf8', flag: 'wx' });
    fs.renameSync(temporary, file);
  } catch (error) {
    try {
      fs.unlinkSync(temporary);
    } catch {
      // best effort cleanup only
    }
    throw error;
  }
}

function summarize(file) {
  return reduceLedger(parseLedgerText(fs.readFileSync(file, 'utf8')));
}

function scan() {
  let entries;
  try {
    entries = fs.readdirSync(INBOX);
  } catch (error) {
    if (error.code === 'ENOENT') return { batches: [], done: false };
    throw error;
  }
  return {
    batches: entries.filter((name) => BATCH_PATTERN.test(name)).sort(compareByCodePoint),
    done: entries.includes('DONE'),
  };
}

async function main() {
  fs.mkdirSync(OUT_DIR, { recursive: true });
  fs.writeFileSync(STARTED, JSON.stringify({ at: Date.now() }), 'utf8');

  const processed = new Map();
  const deadline = Date.now() + MAX_WAIT_MS;
  let doneSeenAt = null;
  let quietScans = 0;

  for (;;) {
    const { batches, done } = scan();
    const pending = batches.filter((name) => !processed.has(name));

    for (const name of pending) {
      const summary = summarize(path.join(INBOX, name));
      processed.set(name, { batch: name, summary });
      process.stderr.write(`processed ${name}\n`);
    }

    if (done) {
      doneSeenAt ??= Date.now();
      if (pending.length === 0) {
        quietScans += 1;
        // Two consecutive quiet scans after DONE guard against a batch landing
        // at the same instant as the marker.
        if (quietScans >= 2) break;
      } else {
        quietScans = 0;
      }
    }

    if (Date.now() > deadline) {
      throw new Error(`timed out waiting for DONE in ${INBOX}`);
    }
    await sleep(POLL_MS);
  }

  const stream = [...processed.values()].sort((a, b) => compareByCodePoint(a.batch, b.batch));
  writeAtomic(STREAM, `${JSON.stringify(stream)}\n`);
  process.stderr.write(
    `wrote ${STREAM} with ${stream.length} batches (DONE seen at ${doneSeenAt})\n`,
  );
}

main().catch((error) => {
  process.stderr.write(`error: ${error.message}\n`);
  process.exitCode = 1;
});
