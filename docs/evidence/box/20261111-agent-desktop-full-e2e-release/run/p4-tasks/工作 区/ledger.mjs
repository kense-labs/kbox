/**
 * Pure ledger reduction logic. No filesystem or process access lives here.
 *
 * Events are objects of the form:
 *   { id: string, account: string, cents: safe nonnegative integer,
 *     kind: "credit" | "debit" }
 */

const EVENT_KEYS = ["id", "account", "cents", "kind"];

export class LedgerError extends Error {
  constructor(message) {
    super(message);
    this.name = "LedgerError";
  }
}

function fail(message) {
  throw new LedgerError(message);
}

function isPlainEventObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

/**
 * Validate one parsed event and return a normalized copy containing only the
 * four allowed fields. Extra or missing fields are rejected.
 */
export function validateEvent(value, label = "event") {
  if (!isPlainEventObject(value)) {
    fail(`${label}: must be a JSON object`);
  }

  const keys = Object.keys(value);
  if (
    keys.length !== EVENT_KEYS.length ||
    !EVENT_KEYS.every((key) => Object.hasOwn(value, key))
  ) {
    fail(`${label}: must contain exactly id, account, cents and kind`);
  }

  const { id, account, cents, kind } = value;

  if (typeof id !== "string" || id.length === 0) {
    fail(`${label}: id must be a nonempty string`);
  }
  if (typeof account !== "string" || account.length === 0) {
    fail(`${label}: account must be a nonempty string`);
  }
  if (!Number.isSafeInteger(cents) || cents < 0) {
    fail(`${label}: cents must be a nonnegative safe integer`);
  }
  if (kind !== "credit" && kind !== "debit") {
    fail(`${label}: kind must be "credit" or "debit"`);
  }

  return { id, account, cents, kind };
}

function sameEvent(a, b) {
  return (
    a.id === b.id &&
    a.account === b.account &&
    a.cents === b.cents &&
    a.kind === b.kind
  );
}

/** Compare two strings by Unicode code point (not UTF-16 code unit). */
export function compareByCodePoint(a, b) {
  const left = Array.from(a);
  const right = Array.from(b);
  const length = Math.min(left.length, right.length);

  for (let index = 0; index < length; index += 1) {
    const aPoint = left[index].codePointAt(0);
    const bPoint = right[index].codePointAt(0);
    if (aPoint !== bPoint) {
      return aPoint < bPoint ? -1 : 1;
    }
  }

  return left.length - right.length;
}

function addSafe(a, b, label) {
  const result = a + b;
  if (!Number.isSafeInteger(result)) {
    fail(`${label}: total exceeds the safe integer range`);
  }
  return result;
}

/**
 * Reduce parsed events.
 *
 * Identical duplicates (same id and all fields) are counted once. Two records
 * with the same id but different content make the whole input invalid.
 *
 * options.account, when supplied, filters the reported accounts and netCents.
 * The event counts always describe the whole input.
 */
export function reduceEvents(events, options = {}) {
  if (
    events === null ||
    events === undefined ||
    typeof events[Symbol.iterator] !== "function"
  ) {
    fail("events must be an iterable of event objects");
  }

  const filterAccount = options.account;
  if (filterAccount !== undefined && typeof filterAccount !== "string") {
    fail("account filter must be a string");
  }

  const byId = new Map();
  let duplicateEvents = 0;

  for (const rawEvent of events) {
    const event = validateEvent(rawEvent);
    const previous = byId.get(event.id);

    if (previous === undefined) {
      byId.set(event.id, event);
    } else if (sameEvent(previous, event)) {
      duplicateEvents += 1;
    } else {
      fail(`conflicting records share id ${JSON.stringify(event.id)}`);
    }
  }

  const balances = new Map();
  for (const event of byId.values()) {
    const current = balances.get(event.account) ?? 0;
    const delta = event.kind === "credit" ? event.cents : -event.cents;
    balances.set(event.account, addSafe(current, delta, "account balance"));
  }

  const allAccounts = Array.from(balances, ([account, balanceCents]) => ({
    account,
    balanceCents,
  })).sort((a, b) => compareByCodePoint(a.account, b.account));

  let netCents = 0;
  for (const entry of allAccounts) {
    netCents = addSafe(netCents, entry.balanceCents, "net balance");
  }

  const accounts =
    filterAccount === undefined
      ? allAccounts
      : allAccounts.filter((entry) => entry.account === filterAccount);

  const filteredNetCents =
    filterAccount === undefined
      ? netCents
      : accounts.reduce(
          (sum, entry) => addSafe(sum, entry.balanceCents, "net balance"),
          0,
        );

  return {
    uniqueEvents: byId.size,
    duplicateEvents,
    accounts,
    netCents: filteredNetCents,
  };
}

/**
 * Parse UTF-8 JSONL text. Blank/whitespace-only lines are ignored. JSON or
 * shape errors are reported with a 1-based line number.
 */
export function parseJsonl(text) {
  if (typeof text !== "string") {
    fail("input must be text");
  }

  // A UTF-8 BOM is permitted at the beginning of the file.
  if (text.charCodeAt(0) === 0xfeff) {
    text = text.slice(1);
  }

  const lines = text.split(/\r\n|\n|\r/);
  const events = [];

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === "") {
      continue;
    }

    let parsed;
    try {
      parsed = JSON.parse(line);
    } catch {
      fail(`line ${index + 1}: malformed JSON`);
    }

    events.push(validateEvent(parsed, `line ${index + 1}`));
  }

  return events;
}

// Friendly aliases for callers that expect a differently named reducer.
export { reduceEvents as reduce };
export default reduceEvents;
