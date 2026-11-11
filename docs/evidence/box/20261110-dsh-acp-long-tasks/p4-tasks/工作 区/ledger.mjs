/**
 * Dependency-free ledger reduction.
 *
 * An event has the shape:
 *   { id: string, account: string, cents: number, kind: "credit" | "debit" }
 *
 * Credits add cents and debits subtract cents.  All balances are accumulated
 * with BigInt internally so the arithmetic never loses precision, then
 * converted back to safe JavaScript integers before being returned.
 */

export class LedgerError extends Error {
  constructor(message) {
    super(message);
    this.name = "LedgerError";
  }
}

const KINDS = new Set(["credit", "debit"]);

function isPlainObject(value) {
  return (
    value !== null &&
    typeof value === "object" &&
    !Array.isArray(value)
  );
}

function validateEvent(event, index) {
  const where = `event ${index + 1}`;
  if (!isPlainObject(event)) {
    throw new LedgerError(`${where} must be a JSON object`);
  }

  if (typeof event.id !== "string" || event.id.length === 0 || event.id.trim() === "") {
    throw new LedgerError(`${where} has an invalid id (expected a nonempty string)`);
  }
  if (
    typeof event.account !== "string" ||
    event.account.length === 0 ||
    event.account.trim() === ""
  ) {
    throw new LedgerError(`${where} has an invalid account (expected a nonempty string)`);
  }
  if (
    typeof event.cents !== "number" ||
    !Number.isSafeInteger(event.cents) ||
    event.cents < 0
  ) {
    throw new LedgerError(
      `${where} has an invalid cents value (expected a nonnegative safe integer)`
    );
  }
  if (typeof event.kind !== "string" || !KINDS.has(event.kind)) {
    throw new LedgerError(`${where} has an invalid kind (expected "credit" or "debit")`);
  }

  return {
    id: event.id,
    account: event.account,
    cents: event.cents,
    kind: event.kind,
  };
}

function sameEvent(a, b) {
  // `-0 === 0` is true; that is intentional for JSON numeric equality.
  return (
    a.id === b.id &&
    a.account === b.account &&
    a.cents === b.cents &&
    a.kind === b.kind
  );
}

/** Compare two strings by Unicode code point (not UTF-16 code unit). */
export function compareByCodePoint(a, b) {
  const left = Array.from(a, (ch) => ch.codePointAt(0));
  const right = Array.from(b, (ch) => ch.codePointAt(0));
  const length = Math.min(left.length, right.length);
  for (let i = 0; i < length; i += 1) {
    if (left[i] !== right[i]) return left[i] - right[i];
  }
  return left.length - right.length;
}

/**
 * Parse UTF-8 JSONL text into event objects.  Blank (whitespace-only) lines
 * are ignored.  Syntax errors and non-object records are rejected.
 */
export function parseJsonl(text) {
  if (typeof text !== "string") {
    throw new LedgerError("input must be text");
  }

  const events = [];
  const lines = text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (line.trim() === "") continue;

    let event;
    try {
      event = JSON.parse(line);
    } catch (error) {
      throw new LedgerError(`malformed JSON on line ${i + 1}: ${error.message}`);
    }
    if (!isPlainObject(event)) {
      throw new LedgerError(`line ${i + 1} must contain a JSON object`);
    }
    events.push(event);
  }
  return events;
}

function normalizeOptions(options) {
  if (options === undefined || options === null) return {};
  if (typeof options === "string") return { account: options };
  if (typeof options !== "object" || Array.isArray(options)) {
    throw new LedgerError("options must be an object or account name");
  }
  return options;
}

/**
 * Reduce events (an iterable of event objects, or JSONL text) to the ledger
 * summary.  `options.account` (or a string second argument) filters the
 * emitted accounts and netCents, but not the event counts.
 *
 * @param {Iterable<object>|string} events
 * @param {{account?: string}|string} [options]
 * @returns {{uniqueEvents: number, duplicateEvents: number,
 *            accounts: Array<{account: string, balanceCents: number}>,
 *            netCents: number}}
 */
export function reduceEvents(events, options) {
  const opts = normalizeOptions(options);

  let records;
  if (typeof events === "string") {
    records = parseJsonl(events);
  } else if (events != null && typeof events[Symbol.iterator] === "function") {
    records = Array.from(events);
  } else {
    throw new LedgerError("events must be an iterable or JSONL text");
  }

  const byId = new Map();
  let duplicateEvents = 0;

  for (let i = 0; i < records.length; i += 1) {
    const event = validateEvent(records[i], i);
    const previous = byId.get(event.id);
    if (previous === undefined) {
      byId.set(event.id, event);
    } else if (sameEvent(previous, event)) {
      duplicateEvents += 1;
    } else {
      throw new LedgerError(
        `conflicting records share id ${JSON.stringify(event.id)}`
      );
    }
  }

  const balances = new Map();
  for (const event of byId.values()) {
    const delta = BigInt(event.cents) * (event.kind === "credit" ? 1n : -1n);
    balances.set(event.account, (balances.get(event.account) ?? 0n) + delta);
  }

  let selected = [...balances.entries()];
  if (opts.account !== undefined) {
    if (typeof opts.account !== "string" || opts.account.length === 0) {
      throw new LedgerError("account filter must be a nonempty string");
    }
    selected = selected.filter(([account]) => account === opts.account);
  }

  selected.sort(([a], [b]) => compareByCodePoint(a, b));

  let net = 0n;
  const accounts = selected.map(([account, balance]) => {
    net += balance;
    return { account, balanceCents: toSafeNumber(balance, `balance for ${account}`) };
  });

  return {
    uniqueEvents: byId.size,
    duplicateEvents,
    accounts,
    netCents: toSafeNumber(net, "net total"),
  };
}

function toSafeNumber(value, label) {
  if (value > BigInt(Number.MAX_SAFE_INTEGER) || value < BigInt(Number.MIN_SAFE_INTEGER)) {
    throw new LedgerError(`${label} exceeds the safe integer range`);
  }
  return Number(value);
}

// Convenient aliases so the reducer can be imported under natural names.
export const reduceLedger = reduceEvents;
export const reconcile = reduceEvents;
export const reducer = reduceEvents;
export const reduce = reduceEvents;
export default reduceEvents;
