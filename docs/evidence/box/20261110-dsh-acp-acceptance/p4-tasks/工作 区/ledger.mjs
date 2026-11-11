/**
 * Dependency-free ledger reducer.
 *
 * A ledger event has the shape:
 *   { id: string, account: string, cents: number, kind: "credit" | "debit" }
 *
 * All arithmetic is done with safe integer Numbers.  Balances that would leave
 * the safe-integer range are rejected instead of producing a rounded result.
 */

export class LedgerError extends Error {
  constructor(message) {
    super(message);
    this.name = "LedgerError";
  }
}

const VALID_KINDS = new Set(["credit", "debit"]);

/**
 * Compare two strings by Unicode code point (not by UTF-16 code unit).
 * JavaScript's default sort compares UTF-16 code units, which orders astral
 * characters differently from their code points.
 */
export function compareCodePoints(a, b) {
  const left = Array.from(a);
  const right = Array.from(b);
  const length = Math.min(left.length, right.length);
  for (let i = 0; i < length; i += 1) {
    const delta = left[i].codePointAt(0) - right[i].codePointAt(0);
    if (delta !== 0) return delta;
  }
  return left.length - right.length;
}

function isPlainObject(value) {
  return (
    value !== null &&
    typeof value === "object" &&
    !Array.isArray(value)
  );
}

function describeLocation(location) {
  return location === undefined ? "" : ` (${location})`;
}

/**
 * Validate one already-JSON-parsed ledger record.
 * Returns a normalized copy containing the four ledger fields.
 */
export function validateEvent(value, location) {
  const where = describeLocation(location);
  if (!isPlainObject(value)) {
    throw new LedgerError(`event${where} must be a JSON object`);
  }

  if (typeof value.id !== "string" || value.id.length === 0) {
    throw new LedgerError(`event${where} has an invalid id: expected a nonempty string`);
  }
  if (typeof value.account !== "string" || value.account.length === 0) {
    throw new LedgerError(`event${where} has an invalid account: expected a nonempty string`);
  }
  if (
    typeof value.cents !== "number" ||
    !Number.isSafeInteger(value.cents) ||
    value.cents < 0
  ) {
    throw new LedgerError(
      `event${where} has invalid cents: expected a nonnegative safe integer`,
    );
  }
  if (typeof value.kind !== "string" || !VALID_KINDS.has(value.kind)) {
    throw new LedgerError(
      `event${where} has an invalid kind: expected "credit" or "debit"`,
    );
  }

  return {
    id: value.id,
    account: value.account,
    cents: value.cents,
    kind: value.kind,
  };
}

function sameEvent(a, b) {
  return (
    a.id === b.id &&
    a.account === b.account &&
    a.cents === b.cents &&
    a.kind === b.kind
  );
}

/**
 * Parse UTF-8 JSONL text into validated records.  Blank/whitespace-only
 * lines are ignored.  No deduplication happens here.
 */
export function parseJsonl(text) {
  if (typeof text !== "string") {
    throw new LedgerError("input must be text");
  }

  // A leading UTF-8 BOM is not part of the first JSON value.
  if (text.charCodeAt(0) === 0xfeff) text = text.slice(1);

  const records = [];
  const lines = text.split("\n");
  for (let index = 0; index < lines.length; index += 1) {
    const rawLine = lines[index];
    if (rawLine.trim() === "") continue;

    let parsed;
    try {
      parsed = JSON.parse(rawLine);
    } catch {
      throw new LedgerError(`line ${index + 1} is not valid JSON`);
    }
    records.push(validateEvent(parsed, `on line ${index + 1}`));
  }
  return records;
}

/**
 * Aggregate validated or raw records.
 *
 * Identical records with the same id are counted as duplicate occurrences.
 * Two records with the same id but different ledger fields make the whole
 * input invalid.
 *
 * options.account, when a string, restricts accounts/netCents to that account.
 * uniqueEvents/duplicateEvents always describe the whole input.
 */
export function reduceEvents(records, options = {}) {
  const accountFilter = options.account;
  if (accountFilter !== undefined && accountFilter !== null && typeof accountFilter !== "string") {
    throw new LedgerError("account filter must be a string");
  }

  const byId = new Map();
  const balances = new Map();
  let duplicateEvents = 0;
  let lineHint = 0;

  for (const rawRecord of records) {
    lineHint += 1;
    const event = validateEvent(rawRecord, `record ${lineHint}`);

    const previous = byId.get(event.id);
    if (previous !== undefined) {
      if (!sameEvent(previous, event)) {
        throw new LedgerError(`conflicting records share id ${JSON.stringify(event.id)}`);
      }
      duplicateEvents += 1;
      continue;
    }
    byId.set(event.id, event);

    const delta = event.kind === "credit" ? event.cents : -event.cents;
    const next = (balances.get(event.account) ?? 0) + delta;
    if (!Number.isSafeInteger(next)) {
      throw new LedgerError(
        `balance overflow for account ${JSON.stringify(event.account)}`,
      );
    }
    balances.set(event.account, next);
  }

  const allAccounts = [...balances.keys()].sort(compareCodePoints);
  let selectedAccounts = allAccounts;
  if (typeof accountFilter === "string") {
    selectedAccounts = allAccounts.filter((account) => account === accountFilter);
  }

  const accounts = selectedAccounts.map((account) => ({
    account,
    balanceCents: balances.get(account),
  }));

  let netCents = 0;
  for (const entry of accounts) {
    const next = netCents + entry.balanceCents;
    if (!Number.isSafeInteger(next)) {
      throw new LedgerError("net balance overflow");
    }
    netCents = next;
  }

  return {
    uniqueEvents: byId.size,
    duplicateEvents,
    accounts,
    netCents,
  };
}

/**
 * Main reducer: UTF-8 JSONL text in, summary object out.
 */
export function reduceLedger(text, options = {}) {
  return reduceEvents(parseJsonl(text), options);
}

// Convenient aliases for callers that expect a shorter "reducer" export.
export const reduce = reduceLedger;
export const reducer = reduceLedger;
export default reduceLedger;
