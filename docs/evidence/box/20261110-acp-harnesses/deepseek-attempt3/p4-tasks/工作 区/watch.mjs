#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import {
  reduceLedger,
  serializeSummary,
} from './ledger.mjs';

const INBOX_DIRECTORY = '/external/inbox';
const OUTPUT_DIRECTORY = '/workspace/out';
const START_MARKER = path.join(OUTPUT_DIRECTORY, 'watch-started.json');
const STREAM_OUTPUT = path.join(OUTPUT_DIRECTORY, 'stream.json');
const POLL_INTERVAL_MS = 250;
const BATCH_PATTERN = /^batch-\d+\.jsonl$/;

function compareNames(left, right) {
  // Filename ordering is deliberately locale-independent.
  return left < right ? -1 : left > right ? 1 : 0;
}

function writeAtomically(filePath, contents) {
  const directory = path.dirname(filePath);
  fs.mkdirSync(directory, { recursive: true });

  const temporary = path.join(
    directory,
    `.${path.basename(filePath)}.${process.pid}.${Date.now()}.${Math.random()
      .toString(16)
      .slice(2)}.tmp`,
  );

  try {
    fs.writeFileSync(temporary, contents, { encoding: 'utf8', flag: 'wx' });
    fs.renameSync(temporary, filePath);
  } catch (error) {
    try {
      fs.unlinkSync(temporary);
    } catch {
      // The temporary file may not have been created.
    }
    throw error;
  }
}

function readInboxNames() {
  try {
    return fs.readdirSync(INBOX_DIRECTORY);
  } catch (error) {
    if (error && error.code === 'ENOENT') {
      return [];
    }
    throw error;
  }
}

function isDoneMarkerPresent(names) {
  return names.includes('DONE');
}

function batchNamesInOrder(names) {
  return names
    .filter((name) => BATCH_PATTERN.test(name))
    .sort(compareNames);
}

function scanBatches(processed) {
  const names = readInboxNames();
  let added = 0;

  for (const batch of batchNamesInOrder(names)) {
    if (processed.has(batch)) {
      continue;
    }

    // The host publishes files with an atomic rename, so a listed batch file
    // is complete. If it nevertheless disappears, retry on the next scan.
    let text;
    try {
      text = fs.readFileSync(path.join(INBOX_DIRECTORY, batch), 'utf8');
    } catch (error) {
      if (error && error.code === 'ENOENT') {
        continue;
      }
      throw error;
    }

    const summary = reduceLedger(text);
    processed.set(batch, summary);
    added += 1;
    process.stdout.write(
      `watch: processed ${batch} (unique=${summary.uniqueEvents}, duplicates=${summary.duplicateEvents})\n`,
    );
  }

  return { added, names };
}

function writeStream(processed) {
  const ordered = [...processed.entries()].sort(([left], [right]) =>
    compareNames(left, right),
  );

  const entries = ordered
    .map(
      ([batch, summary]) =>
        `{"batch":${JSON.stringify(batch)},"summary":${serializeSummary(
          summary,
        )}}`,
    )
    .join(',');

  writeAtomically(STREAM_OUTPUT, `[${entries}]\n`);
}

const sleep = (milliseconds) =>
  new Promise((resolve) => setTimeout(resolve, milliseconds));

async function main() {
  fs.mkdirSync(OUTPUT_DIRECTORY, { recursive: true });

  // The host waits for this marker before beginning the 125-second delivery
  // window, so write it before doing any inbox scanning.
  writeAtomically(START_MARKER, JSON.stringify({ at: Date.now() }));

  const processed = new Map();
  let doneSeen = false;

  for (;;) {
    const { names } = scanBatches(processed);

    if (doneSeen) {
      break;
    }

    if (isDoneMarkerPresent(names) || isDoneMarkerPresent(readInboxNames())) {
      doneSeen = true;
      // One final scan after observing DONE guarantees that every batch
      // delivered before DONE is included.
      scanBatches(processed);
      break;
    }

    await sleep(POLL_INTERVAL_MS);
  }

  writeStream(processed);
  process.stdout.write(
    `watch: wrote ${STREAM_OUTPUT} with ${processed.size} batch(es)\n`,
  );
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`watch: ${message}\n`);
  process.exitCode = 1;
});
