#!/usr/bin/env node
// CLI: node reconcile.mjs INPUT OUTPUT [--account NAME]
//
// Reads UTF-8 JSONL events, reduces them, and atomically replaces OUTPUT.
// Invalid input exits nonzero before OUTPUT is touched.

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { LedgerError, parseLedgerText, reduceLedger } from './ledger.mjs';

const USAGE = 'usage: node reconcile.mjs INPUT OUTPUT [--account NAME]';

class UsageError extends Error {}

function parseArgs(argv) {
  const positional = [];
  let account;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--account') {
      if (account !== undefined) throw new UsageError('--account given more than once');
      const value = argv[i + 1];
      if (value === undefined) throw new UsageError('--account requires a value');
      if (value.length === 0) throw new UsageError('--account requires a nonempty value');
      account = value;
      i += 1;
    } else if (arg.startsWith('-') && arg !== '-') {
      throw new UsageError(`unknown option ${JSON.stringify(arg)}`);
    } else {
      positional.push(arg);
    }
  }

  if (positional.length !== 2) {
    throw new UsageError(`expected 2 positional arguments, got ${positional.length}`);
  }
  return { input: positional[0], output: positional[1], account };
}

function readInput(file) {
  try {
    return fs.readFileSync(file, 'utf8');
  } catch (error) {
    throw new LedgerError(`cannot read input ${JSON.stringify(file)}: ${error.code ?? error.message}`);
  }
}

/** Write atomically: temp file in the target directory, then rename over OUTPUT. */
function writeOutputAtomically(file, contents) {
  const target = path.resolve(file);
  const directory = path.dirname(target);
  fs.mkdirSync(directory, { recursive: true });
  const temporary = path.join(
    directory,
    `.${path.basename(target)}.${process.pid}.${crypto.randomBytes(6).toString('hex')}.tmp`,
  );
  try {
    fs.writeFileSync(temporary, contents, { encoding: 'utf8', flag: 'wx' });
    fs.renameSync(temporary, target);
  } catch (error) {
    try {
      fs.unlinkSync(temporary);
    } catch {
      // best effort cleanup only
    }
    throw new LedgerError(`cannot write output ${JSON.stringify(file)}: ${error.code ?? error.message}`);
  }
}

export function run(argv) {
  const { input, output, account } = parseArgs(argv);
  const text = readInput(input);
  const records = parseLedgerText(text);
  const result = reduceLedger(records, account === undefined ? {} : { account });
  writeOutputAtomically(output, `${JSON.stringify(result)}\n`);
  return result;
}

function main() {
  try {
    run(process.argv.slice(2));
    return 0;
  } catch (error) {
    if (error instanceof UsageError) {
      process.stderr.write(`error: ${error.message}\n${USAGE}\n`);
      return 2;
    }
    const message = error instanceof LedgerError ? error.message : `unexpected error: ${error.message}`;
    process.stderr.write(`error: ${message}\n`);
    return 1;
  }
}

if (import.meta.url === `file://${process.argv[1]}` || process.argv[1]?.endsWith('reconcile.mjs')) {
  process.exitCode = main();
}
