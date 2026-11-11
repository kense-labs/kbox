#!/usr/bin/env node
// CLI for the dependency-free ledger reducer.

import {
  closeSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { formatReport, LedgerError, reduceLedger } from './ledger.mjs';

const USAGE = 'usage: node reconcile.mjs INPUT OUTPUT [--account NAME]';

export function parseArgs(argv) {
  const positional = [];
  let account;
  let endOfOptions = false;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];

    if (!endOfOptions && arg === '--') {
      endOfOptions = true;
      continue;
    }

    if (!endOfOptions && arg === '--account') {
      if (i + 1 >= argv.length) {
        throw new LedgerError('--account requires a value', { code: 'USAGE' });
      }
      account = argv[i + 1];
      i += 1;
      continue;
    }

    if (!endOfOptions && arg.startsWith('--account=')) {
      account = arg.slice('--account='.length);
      continue;
    }

    if (!endOfOptions && arg.startsWith('-') && arg !== '-') {
      throw new LedgerError(`Unknown option: ${arg}`, { code: 'USAGE' });
    }

    positional.push(arg);
  }

  if (positional.length !== 2) {
    throw new LedgerError(USAGE, { code: 'USAGE' });
  }

  return { input: positional[0], output: positional[1], account };
}

function readUtf8File(filePath) {
  const bytes = readFileSync(filePath);
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    throw new LedgerError(`Input is not valid UTF-8: ${filePath}`, {
      code: 'INVALID_UTF8',
    });
  }
}

/**
 * Write a file atomically: the destination is only replaced after the complete
 * new content has been written and flushed to a temporary file in the same
 * directory. A failure leaves any existing destination untouched.
 */
export function writeFileAtomic(destination, content) {
  const target = resolve(destination);
  const directory = dirname(target);
  mkdirSync(directory, { recursive: true });

  const temporary = resolve(
    directory,
    `.${basename(target)}.${process.pid}.${Date.now()}.${Math.random().toString(16).slice(2)}.tmp`,
  );

  let fd;
  try {
    fd = openSync(temporary, 'wx', 0o666);
    writeFileSync(fd, content, { encoding: 'utf8' });
    fsyncSync(fd);
    closeSync(fd);
    fd = undefined;
    renameSync(temporary, target);
  } catch (error) {
    if (fd !== undefined) {
      try { closeSync(fd); } catch { /* best effort */ }
    }
    try { unlinkSync(temporary); } catch { /* best effort */ }
    throw error;
  }
}

export function runCli(argv) {
  const { input, output, account } = parseArgs(argv);
  const text = readUtf8File(input);
  const report = reduceLedger(text, account === undefined ? {} : { account });
  writeFileAtomic(output, formatReport(report));
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
  try {
    runCli(process.argv.slice(2));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`reconcile: ${message}`);
    process.exitCode = 1;
  }
}
