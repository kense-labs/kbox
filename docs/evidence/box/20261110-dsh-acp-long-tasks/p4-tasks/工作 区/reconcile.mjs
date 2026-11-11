#!/usr/bin/env node
/**
 * Ledger reconciliation CLI.
 *
 * Usage: node reconcile.mjs INPUT OUTPUT [--account NAME]
 *
 * INPUT is UTF-8 JSONL.  The output file is only replaced after the entire
 * input has been read, parsed, validated and reduced successfully.
 */

import {
  closeSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { LedgerError, parseJsonl, reduceEvents } from "./ledger.mjs";

const USAGE = "usage: node reconcile.mjs INPUT OUTPUT [--account NAME]";

export function parseArgs(argv) {
  let input;
  let output;
  let account;
  let hasAccount = false;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];

    if (arg === "--account" || arg.startsWith("--account=")) {
      if (hasAccount) throw new LedgerError("--account may only be given once");
      let value;
      if (arg === "--account") {
        if (i + 1 >= argv.length) {
          throw new LedgerError("--account requires a value");
        }
        value = argv[i + 1];
        i += 1;
      } else {
        value = arg.slice("--account=".length);
      }
      if (value.length === 0) {
        throw new LedgerError("--account requires a nonempty value");
      }
      account = value;
      hasAccount = true;
    } else if (arg === "--help" || arg === "-h") {
      return { help: true };
    } else if (arg.startsWith("-")) {
      throw new LedgerError(`unknown option ${JSON.stringify(arg)}`);
    } else if (input === undefined) {
      input = arg;
    } else if (output === undefined) {
      output = arg;
    } else {
      throw new LedgerError("too many positional arguments");
    }
  }

  if (input === undefined || output === undefined) {
    throw new LedgerError("both INPUT and OUTPUT are required");
  }
  if (input.length === 0 || output.length === 0) {
    throw new LedgerError("INPUT and OUTPUT must be nonempty paths");
  }

  return { input, output, account };
}

/**
 * Write to a temporary file in the destination directory and atomically
 * rename it over OUTPUT.  If anything fails, OUTPUT is left untouched.
 */
export function writeOutputAtomically(outputPath, contents) {
  const directory = dirname(outputPath);
  mkdirSync(directory, { recursive: true });

  const temporary = join(
    directory,
    `.${basename(outputPath)}.tmp-${process.pid}-${Date.now()}-${Math.random()
      .toString(36)
      .slice(2)}`
  );

  let fd;
  try {
    fd = openSync(temporary, "wx", 0o666);
    writeFileSync(fd, contents, { encoding: "utf8" });
    closeSync(fd);
    fd = undefined;
    renameSync(temporary, outputPath);
  } catch (error) {
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch {
        // Ignore secondary close errors.
      }
    }
    try {
      unlinkSync(temporary);
    } catch {
      // The temporary file may never have been created.
    }
    throw error;
  }
}

export function run(argv) {
  const args = parseArgs(argv);
  if (args.help) {
    process.stdout.write(`${USAGE}\n`);
    return 0;
  }

  // Read and validate everything before touching OUTPUT.
  let text;
  try {
    text = readFileSync(args.input, "utf8");
  } catch (error) {
    throw new LedgerError(`cannot read input ${JSON.stringify(args.input)}: ${error.message}`);
  }

  const result = reduceEvents(parseJsonl(text), {
    ...(args.account === undefined ? {} : { account: args.account }),
  });

  writeOutputAtomically(args.output, `${JSON.stringify(result, null, 2)}\n`);
  return 0;
}

export function main(argv = process.argv.slice(2)) {
  try {
    return run(argv);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(`error: ${message}\n`);
    return 1;
  }
}

const invokedDirectly =
  process.argv[1] !== undefined &&
  fileURLToPath(import.meta.url) === resolve(process.argv[1]);

if (invokedDirectly) {
  process.exitCode = main();
}
