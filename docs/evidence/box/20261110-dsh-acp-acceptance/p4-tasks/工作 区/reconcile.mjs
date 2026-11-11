#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { TextDecoder } from "node:util";

import { LedgerError, reduceLedger } from "./ledger.mjs";

const USAGE = "usage: node reconcile.mjs INPUT OUTPUT [--account NAME]";

function parseArgs(argv) {
  const positional = [];
  let account = null;

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];

    if (argument === "--account") {
      if (account !== null) {
        throw new LedgerError("--account may only be specified once");
      }
      if (index + 1 >= argv.length) {
        throw new LedgerError("--account requires a name");
      }
      account = argv[index + 1];
      index += 1;
      if (account.length === 0) {
        throw new LedgerError("--account requires a nonempty name");
      }
    } else if (argument.startsWith("--account=")) {
      if (account !== null) {
        throw new LedgerError("--account may only be specified once");
      }
      account = argument.slice("--account=".length);
      if (account.length === 0) {
        throw new LedgerError("--account requires a nonempty name");
      }
    } else if (argument === "--") {
      for (let rest = index + 1; rest < argv.length; rest += 1) {
        positional.push(argv[rest]);
      }
      break;
    } else if (argument.startsWith("--")) {
      throw new LedgerError(`unknown option ${JSON.stringify(argument)}`);
    } else {
      positional.push(argument);
    }
  }

  if (positional.length !== 2) {
    throw new LedgerError(USAGE);
  }

  return { input: positional[0], output: positional[1], account };
}

function decodeUtf8(buffer) {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
  } catch {
    throw new LedgerError("input is not valid UTF-8");
  }
}

async function writeOutputAtomically(outputPath, contents) {
  const resolved = path.resolve(outputPath);
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
      // Best-effort cleanup; the original output was never touched.
    }
    throw error;
  }
}

export async function run(argv = process.argv.slice(2)) {
  const { input, output, account } = parseArgs(argv);

  let text;
  try {
    text = decodeUtf8(await readFile(input));
  } catch (error) {
    if (error instanceof LedgerError) throw error;
    throw new LedgerError(`cannot read input ${JSON.stringify(input)}: ${error.message}`);
  }

  const result = reduceLedger(text, account === null ? {} : { account });

  try {
    await writeOutputAtomically(output, `${JSON.stringify(result)}\n`);
  } catch (error) {
    throw new LedgerError(`cannot write output ${JSON.stringify(output)}: ${error.message}`);
  }
}

export async function main(argv = process.argv.slice(2)) {
  try {
    await run(argv);
    return 0;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(`reconcile: ${message}\n`);
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
