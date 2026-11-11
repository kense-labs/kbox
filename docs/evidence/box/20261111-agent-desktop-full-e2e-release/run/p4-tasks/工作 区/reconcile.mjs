#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { readFile, mkdir, writeFile, rename, unlink } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  LedgerError,
  parseJsonl,
  reduceEvents,
} from "./ledger.mjs";

const USAGE = "usage: node reconcile.mjs INPUT OUTPUT [--account NAME]";

function fail(message) {
  throw new LedgerError(message);
}

/**
 * Parse CLI arguments. Positional INPUT/OUTPUT may appear on either side of
 * the optional --account flag.
 */
export function parseArgs(argv) {
  let input;
  let output;
  let account;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--account") {
      if (account !== undefined) {
        fail("--account may only be supplied once");
      }
      index += 1;
      if (index >= argv.length) {
        fail(`--account requires a name\n${USAGE}`);
      }
      account = argv[index];
    } else if (arg.startsWith("--account=")) {
      if (account !== undefined) {
        fail("--account may only be supplied once");
      }
      account = arg.slice("--account=".length);
    } else if (arg.startsWith("-") && arg !== "-") {
      fail(`unknown option ${JSON.stringify(arg)}\n${USAGE}`);
    } else if (input === undefined) {
      input = arg;
    } else if (output === undefined) {
      output = arg;
    } else {
      fail(`unexpected argument ${JSON.stringify(arg)}\n${USAGE}`);
    }
  }

  if (input === undefined || output === undefined) {
    fail(`INPUT and OUTPUT are required\n${USAGE}`);
  }

  return { input, output, account };
}

async function readUtf8File(path) {
  const buffer = await readFile(path);
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
  } catch {
    fail(`${path}: input is not valid UTF-8`);
  }
}

/**
 * Write the rendered JSON next to OUTPUT and atomically rename it into place.
 * A failure before the rename leaves an existing OUTPUT untouched.
 */
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

/**
 * Run the CLI. Returns normally on success; throws LedgerError (or a
 * filesystem error) on failure.
 */
export async function main(argv = process.argv.slice(2)) {
  const { input, output, account } = parseArgs(argv);

  // Read and fully validate everything before touching OUTPUT.
  const text = await readUtf8File(input);
  const events = parseJsonl(text);
  const result = reduceEvents(events, { account });

  const rendered = `${JSON.stringify(result, null, 2)}\n`;
  await writeAtomic(output, rendered);
}

function isDirectRun() {
  const invokedPath = process.argv[1];
  return (
    typeof invokedPath === "string" &&
    resolve(invokedPath) === fileURLToPath(import.meta.url)
  );
}

if (isDirectRun()) {
  main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(`reconcile: ${message}\n`);
    process.exitCode = 1;
  });
}
