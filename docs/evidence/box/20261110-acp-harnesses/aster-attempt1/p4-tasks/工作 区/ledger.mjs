// Dependency-free ledger reducer.
//
// An event is {id, account, cents, kind} where
//   id      nonempty string
//   account nonempty string
//   cents   nonnegative safe integer
//   kind    "credit" | "debit"
//
// Identical events sharing an id are counted as duplicates; records sharing an
// id but disagreeing on any canonical field are a hard error.

const KINDS = new Set(['credit', 'debit']);

export class LedgerError extends Error {
  constructor(message) {
    super(message);
    this.name = 'LedgerError';
  }
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function describeLine(lineNumber) {
  return lineNumber === undefined ? '' : `line ${lineNumber}: `;
}

function requireNonEmptyString(value, field, lineNumber) {
  if (typeof value !== 'string' || value.length === 0) {
    throw new LedgerError(`${describeLine(lineNumber)}${field} must be a nonempty string`);
  }
  return value;
}

/**
 * Validate one parsed JSON value and return its canonical event.
 * Unknown extra fields are ignored; only the canonical fields define identity.
 */
export function validateEvent(record, lineNumber) {
  if (!isRecord(record)) {
    throw new LedgerError(`${describeLine(lineNumber)}event must be a JSON object`);
  }
  const id = requireNonEmptyString(record.id, 'id', lineNumber);
  const account = requireNonEmptyString(record.account, 'account', lineNumber);
  const { cents } = record;
  if (!Number.isSafeInteger(cents) || cents < 0) {
    throw new LedgerError(
      `${describeLine(lineNumber)}cents must be a nonnegative safe integer (event ${JSON.stringify(id)})`,
    );
  }
  if (typeof record.kind !== 'string' || !KINDS.has(record.kind)) {
    throw new LedgerError(
      `${describeLine(lineNumber)}kind must be "credit" or "debit" (event ${JSON.stringify(id)})`,
    );
  }
  return { id, account, cents, kind: record.kind };
}

/** Parse UTF-8 JSONL text into raw event objects. Blank lines are ignored. */
export function parseLedgerText(text) {
  const source = text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  const records = [];
  const lines = source.split(/\r\n|\n|\r/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === '') continue;
    let parsed;
    try {
      parsed = JSON.parse(line);
    } catch {
      throw new LedgerError(`line ${index + 1}: invalid JSON`);
    }
    records.push({ record: parsed, lineNumber: index + 1 });
  }
  return records;
}

/**
 * Compare two strings by Unicode code point (not UTF-16 code unit).
 */
export function compareByCodePoint(a, b) {
  const left = Array.from(a);
  const right = Array.from(b);
  const shared = Math.min(left.length, right.length);
  for (let i = 0; i < shared; i += 1) {
    const lc = left[i].codePointAt(0);
    const rc = right[i].codePointAt(0);
    if (lc !== rc) return lc < rc ? -1 : 1;
  }
  return left.length === right.length ? 0 : left.length < right.length ? -1 : 1;
}

function addCents(total, cents, label) {
  const next = total + cents;
  if (!Number.isSafeInteger(next)) {
    throw new LedgerError(`integer overflow while totalling ${label}`);
  }
  return next;
}

/**
 * The reducer: validate, deduplicate and total events.
 *
 * @param {Array<object>} records raw events, or {record, lineNumber} wrappers
 * @param {{account?: string}} [options]
 * @returns {{uniqueEvents: number, duplicateEvents: number,
 *            accounts: Array<{account: string, balanceCents: number}>, netCents: number}}
 */
export function reduceLedger(records, options = {}) {
  if (!Array.isArray(records)) {
    throw new LedgerError('records must be an array');
  }
  const filter = options.account;
  if (filter !== undefined && (typeof filter !== 'string' || filter.length === 0)) {
    throw new LedgerError('account filter must be a nonempty string');
  }

  const byId = new Map();
  const balances = new Map();
  let duplicateEvents = 0;

  for (const entry of records) {
    const wrapped = isRecord(entry) && 'record' in entry && 'lineNumber' in entry;
    const event = validateEvent(wrapped ? entry.record : entry, wrapped ? entry.lineNumber : undefined);

    const prior = byId.get(event.id);
    if (prior !== undefined) {
      if (prior.account !== event.account || prior.cents !== event.cents || prior.kind !== event.kind) {
        throw new LedgerError(`conflicting records share id ${JSON.stringify(event.id)}`);
      }
      duplicateEvents += 1;
      continue;
    }
    byId.set(event.id, event);

    const delta = event.kind === 'credit' ? event.cents : -event.cents;
    const previous = balances.get(event.account) ?? 0;
    balances.set(event.account, addCents(previous, delta, `account ${JSON.stringify(event.account)}`));
  }

  const selected = [];
  let netCents = 0;
  for (const [account, balanceCents] of balances) {
    if (filter !== undefined && account !== filter) continue;
    selected.push({ account, balanceCents });
    netCents = addCents(netCents, balanceCents, 'net total');
  }
  selected.sort((a, b) => compareByCodePoint(a.account, b.account));

  return {
    uniqueEvents: byId.size,
    duplicateEvents,
    accounts: selected,
    netCents,
  };
}

export default reduceLedger;
