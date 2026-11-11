#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import {
  LedgerError,
  reduceLedger,
  serializeSummary,
} from './ledger.mjs';

const USAGE =
  'Usage: node reconcile.mjs INPUT OUTPUT [--account NAME]';

function parseArgs(argv) {
  let input;
  let output;
  let account;

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];

    if (argument === '--account') {
      if (index + 1 >= argv.length || argv[index + 1].startsWith('--')) {
        throw new LedgerError('--account requires a nonempty NAME');
      }
      account = argv[index + 1];
      index += 1;
      continue;
    }

    if (argument.startsWith('--account=')) {
      account = argument.slice('--account='.length);
      continue;
    }

    if (argument === '--help' || argument === '-h') {
      return { help: true };
    }

    if (argument.startsWith('-')) {
      throw new LedgerError(`unknown option: ${argument}`);
    }

    if (input === undefined) {
      input = argument;
    } else if (output === undefined) {
      output = argument;
    } else {
      throw new LedgerError(`unexpected extra argument: ${argument}`);
    }
  }

  if (input === undefined || output === undefined) {
    throw new LedgerError('INPUT and OUTPUT are required');
  }
  if (account !== undefined && account.length === 0) {
    throw new LedgerError('--account requires a nonempty NAME');
  }

  return { input, output, account };
}

function writeOutputAtomically(outputPath, contents) {
  const absoluteOutput = path.resolve(outputPath);
  const parent = path.dirname(absoluteOutput);
  fs.mkdirSync(parent, { recursive: true });

  const temporary = path.join(
    parent,
    `.${path.basename(absoluteOutput)}.${process.pid}.${Date.now()}.${Math.random()
      .toString(16)
      .slice(2)}.tmp`,
  );

  try {
    fs.writeFileSync(temporary, contents, { encoding: 'utf8', flag: 'wx' });
    fs.renameSync(temporary, absoluteOutput);
  } catch (error) {
    try {
      fs.unlinkSync(temporary);
    } catch {
      // The temporary file may not have been created.
    }
    throw error;
  }
}

function main(argv) {
  try {
    const args = parseArgs(argv);

    if (args.help) {
      process.stdout.write(`${USAGE}\n`);
      return;
    }

    // Read and fully validate/reduce before touching OUTPUT. An invalid input
    // therefore cannot destroy an existing output file.
    const text = fs.readFileSync(args.input, 'utf8');
    const summary = reduceLedger(text, { account: args.account });
    writeOutputAtomically(args.output, `${serializeSummary(summary)}\n`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(`reconcile: ${message}\n`);
    process.exitCode = 1;
  }
}

main(process.argv.slice(2));
