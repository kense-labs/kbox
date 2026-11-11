/**
 * Dependency-free ledger core.
 *
 * The public reducer accepts either raw UTF-8 JSONL text or an array of event
 * objects and returns the canonical summary required by the CLI.
 */

export class LedgerError extends Error {
  constructor(message) {
    super(message);
    this.name = 'LedgerError';
  }
}

const KINDS = new Set(['credit', 'debit']);

/** Compare two strings by Unicode code point (not UTF-16 code unit). */
export function compareCodePoints(a, b) {
  const ac = Array.from(a, (character) => character.codePointAt(0));
  const bc = Array.from(b, (character) => character.codePointAt(0));
  const length = Math.min(ac.length, bc.length);

  for (let index = 0; index < length; index += 1) {
    if (ac[index] !== bc[index]) {
      return ac[index] < bc[index] ? -1 : 1;
    }
  }

  return ac.length === bc.length ? 0 : ac.length < bc.length ? -1 : 1;
}

/** Validate one event record and return only its canonical fields. */
export function validateEvent(value, context = 'event') {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new LedgerError(`${context}: record must be a JSON object`);
  }

  const { id, account, cents, kind } = value;

  if (typeof id !== 'string' || id.length === 0) {
    throw new LedgerError(`${context}: id must be a nonempty string`);
  }
  if (typeof account !== 'string' || account.length === 0) {
    throw new LedgerError(`${context}: account must be a nonempty string`);
  }
  if (typeof cents !== 'number' || !Number.isSafeInteger(cents) || cents < 0) {
    throw new LedgerError(
      `${context}: cents must be a nonnegative safe integer`,
    );
  }
  if (typeof kind !== 'string' || !KINDS.has(kind)) {
    throw new LedgerError(`${context}: kind must be "credit" or "debit"`);
  }

  return { id, account, cents, kind };
}

/** Parse UTF-8 JSONL text. Blank lines are ignored. */
export function parseJsonl(text) {
  if (typeof text !== 'string') {
    throw new LedgerError('input must be UTF-8 JSONL text');
  }

  const records = [];
  const lines = text.split(/\r\n|\n|\r/);

  for (let index = 0; index < lines.length; index += 1) {
    let line = lines[index];

    if (index === 0 && line.charCodeAt(0) === 0xfeff) {
      line = line.slice(1);
    }
    if (line.trim() === '') {
      continue;
    }

    let value;
    try {
      value = JSON.parse(line);
    } catch (error) {
      throw new LedgerError(
        `line ${index + 1}: malformed JSON: ${error.message}`,
      );
    }

    records.push(validateEvent(value, `line ${index + 1}`));
  }

  return records;
}

function toCentsValue(value) {
  const max = BigInt(Number.MAX_SAFE_INTEGER);
  const min = -max;
  if (value <= max && value >= min) {
    return Number(value);
  }
  // Keep exact precision for unusually large totals; the serializer emits
  // them as unquoted JSON integers.
  return value;
}

/**
 * Collapse events into the ledger summary.
 *
 * @param {string|Array<object>} input JSONL text or event objects.
 * @param {{account?: string}|string} [options] Optional account filter.
 */
export function reduceLedger(input, options = {}) {
  let events;

  if (typeof input === 'string') {
    events = parseJsonl(input);
  } else if (Array.isArray(input)) {
    events = input.map((event, index) =>
      validateEvent(event, `event ${index + 1}`),
    );
  } else {
    throw new LedgerError('expected JSONL text or an array of events');
  }

  let accountFilter;
  if (typeof options === 'string') {
    accountFilter = options;
  } else if (options !== null && typeof options === 'object') {
    accountFilter = options.account;
  } else if (options !== undefined) {
    throw new LedgerError('options must be an object or an account filter');
  }

  if (
    accountFilter !== undefined &&
    (typeof accountFilter !== 'string' || accountFilter.length === 0)
  ) {
    throw new LedgerError('account filter must be a nonempty string');
  }

  const seen = new Map();
  const balances = new Map();
  let duplicateEvents = 0;
  let net = 0n;

  for (const event of events) {
    const previous = seen.get(event.id);

    if (previous !== undefined) {
      if (
        previous.account !== event.account ||
        previous.cents !== event.cents ||
        previous.kind !== event.kind
      ) {
        throw new LedgerError(
          `conflicting records for id ${JSON.stringify(event.id)}`,
        );
      }
      duplicateEvents += 1;
      continue;
    }

    seen.set(event.id, event);

    const delta = event.kind === 'credit' ? BigInt(event.cents) : -BigInt(event.cents);
    balances.set(event.account, (balances.get(event.account) ?? 0n) + delta);
    net += delta;
  }

  let accountEntries = [...balances.entries()];
  let netTotal = net;

  if (accountFilter !== undefined) {
    accountEntries = accountEntries.filter(
      ([account]) => account === accountFilter,
    );
    netTotal = accountEntries.reduce(
      (total, [, balance]) => total + balance,
      0n,
    );
  }

  accountEntries.sort(([left], [right]) => compareCodePoints(left, right));

  return {
    uniqueEvents: seen.size,
    duplicateEvents,
    accounts: accountEntries.map(([account, balanceCents]) => ({
      account,
      balanceCents: toCentsValue(balanceCents),
    })),
    netCents: toCentsValue(netTotal),
  };
}

function formatCents(value) {
  if (typeof value === 'bigint') {
    return value.toString();
  }
  if (typeof value === 'number' && Number.isSafeInteger(value)) {
    return String(value);
  }
  throw new LedgerError(`cannot serialize non-integer cents value: ${value}`);
}

/** Serialize exactly the required output shape, preserving integer precision. */
export function serializeSummary(summary) {
  const accounts = summary.accounts
    .map(
      ({ account, balanceCents }) =>
        `{"account":${JSON.stringify(account)},"balanceCents":${formatCents(
          balanceCents,
        )}}`,
    )
    .join(',');

  return `{"uniqueEvents":${summary.uniqueEvents},"duplicateEvents":${summary.duplicateEvents},"accounts":[${accounts}],"netCents":${formatCents(
    summary.netCents,
  )}}`;
}

// Friendly aliases make the reducer easy to consume from tests or other code.
export const reducer = reduceLedger;
export const reduce = reduceLedger;
export const reduceEvents = reduceLedger;
export default reduceLedger;
