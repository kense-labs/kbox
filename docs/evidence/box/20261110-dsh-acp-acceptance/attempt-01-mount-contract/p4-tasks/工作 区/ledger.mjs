// Dependency-free ledger reducer.
//
// The reducer accepts either a JSONL string or an iterable of event records.
// It validates every record, removes exact duplicate ids, rejects conflicting
// ids, and accumulates balances with BigInt so that cents are never rounded.

export class LedgerError extends Error {
  constructor(message, options = {}) {
    super(message);
    this.name = 'LedgerError';
    this.code = options.code ?? 'INVALID_LEDGER_INPUT';
    if (options.line !== undefined) this.line = options.line;
  }
}

/** Compare two strings by Unicode code point (not UTF-16 code unit). */
export function compareByCodePoint(a, b) {
  const left = Array.from(a);
  const right = Array.from(b);
  const length = Math.min(left.length, right.length);
  for (let i = 0; i < length; i += 1) {
    const diff = left[i].codePointAt(0) - right[i].codePointAt(0);
    if (diff !== 0) return diff;
  }
  return left.length - right.length;
}

function normalizeEvent(value, line) {
  const where = line === undefined ? '' : ` at line ${line}`;

  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new LedgerError(`Event${where} must be a JSON object`, { line });
  }

  const { id, account, cents, kind } = value;

  if (typeof id !== 'string' || id.length === 0) {
    throw new LedgerError(`Event${where} has an invalid id (expected a nonempty string)`, { line });
  }
  if (typeof account !== 'string' || account.length === 0) {
    throw new LedgerError(`Event${where} has an invalid account (expected a nonempty string)`, { line });
  }
  if (typeof cents !== 'number' || !Number.isSafeInteger(cents) || cents < 0) {
    throw new LedgerError(
      `Event${where} has an invalid cents value (expected a nonnegative safe integer)`,
      { line },
    );
  }
  if (kind !== 'credit' && kind !== 'debit') {
    throw new LedgerError(`Event${where} has an invalid kind (expected "credit" or "debit")`, { line });
  }

  return Object.freeze({ id, account, cents, kind });
}

function sameLedgerRecord(a, b) {
  return a.id === b.id
    && a.account === b.account
    && a.cents === b.cents
    && a.kind === b.kind;
}

function* recordsFromString(text) {
  if (typeof text !== 'string') {
    throw new LedgerError('JSONL input must be a string');
  }

  // A UTF-8 BOM is only special at the very beginning of a file.
  const source = text.startsWith('\uFEFF') ? text.slice(1) : text;
  const lines = source.split(/\r\n|\n|\r/);

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === '') continue;

    let parsed;
    try {
      parsed = JSON.parse(line);
    } catch {
      throw new LedgerError(`Malformed JSON at line ${index + 1}`, {
        code: 'MALFORMED_JSON',
        line: index + 1,
      });
    }

    yield normalizeEvent(parsed, index + 1);
  }
}

/**
 * Parse and validate JSONL into event records, retaining line information.
 * Exported because the CLI needs the same exact validation path as the reducer.
 */
export function parseJsonl(text) {
  return Array.from(recordsFromString(text));
}

function toOutputInteger(value) {
  const min = BigInt(Number.MIN_SAFE_INTEGER);
  const max = BigInt(Number.MAX_SAFE_INTEGER);
  return value >= min && value <= max ? Number(value) : value;
}

/**
 * Reduce ledger events.
 *
 * @param {string|Iterable<object>} input JSONL text or an iterable of events.
 * @param {{account?: string}|string} [options] Optional account filter.
 * @returns {{uniqueEvents: number, duplicateEvents: number,
 *            accounts: Array<{account: string, balanceCents: number|bigint}>,
 *            netCents: number|bigint}}
 *
 * Values that are safe integers are returned as numbers. A balance or net that
 * cannot be represented exactly as a JS safe integer is returned as a BigInt so
 * that no precision is silently lost.
 */
export function reduceLedger(input, options = {}) {
  const filter = typeof options === 'string'
    ? options
    : options?.account;

  if (filter !== undefined && typeof filter !== 'string') {
    throw new LedgerError('Account filter must be a string');
  }

  const records = typeof input === 'string'
    ? recordsFromString(input)
    : (() => {
        if (input === null || typeof input !== 'object' || typeof input[Symbol.iterator] !== 'function') {
          throw new LedgerError('Ledger input must be JSONL text or an iterable of events');
        }
        return (function* iterate() {
          let index = 0;
          for (const value of input) {
            index += 1;
            yield normalizeEvent(value, index);
          }
        })();
      })();

  const seen = new Map();
  const balances = new Map();
  let uniqueEvents = 0;
  let duplicateEvents = 0;

  for (const record of records) {
    const previous = seen.get(record.id);

    if (previous !== undefined) {
      if (!sameLedgerRecord(previous, record)) {
        throw new LedgerError(
          `Conflicting records share id ${JSON.stringify(record.id)}`,
          { code: 'CONFLICTING_ID' },
        );
      }
      duplicateEvents += 1;
      continue;
    }

    seen.set(record.id, record);
    uniqueEvents += 1;

    const current = balances.get(record.account) ?? 0n;
    const amount = BigInt(record.cents);
    balances.set(
      record.account,
      record.kind === 'credit' ? current + amount : current - amount,
    );
  }

  const accountNames = Array.from(balances.keys())
    .filter((account) => filter === undefined || account === filter)
    .sort(compareByCodePoint);

  let net = 0n;
  const accounts = accountNames.map((account) => {
    const balance = balances.get(account);
    net += balance;
    return { account, balanceCents: toOutputInteger(balance) };
  });

  return {
    uniqueEvents,
    duplicateEvents,
    accounts,
    netCents: toOutputInteger(net),
  };
}

/** Aliases make the reducer convenient to import under common names. */
export const reduce = reduceLedger;
export const reduceEvents = reduceLedger;
export const reducer = reduceLedger;

export default reduceLedger;

function integerJson(value) {
  if (typeof value === 'bigint') return value.toString();
  if (typeof value === 'number' && Number.isFinite(value)) return String(value);
  throw new LedgerError('Ledger result contains a non-integer value');
}

/** Serialize a reducer result to the exact report JSON shape. */
export function formatReport(report) {
  const accounts = report.accounts
    .map(({ account, balanceCents }) => (
      `{"account":${JSON.stringify(account)},"balanceCents":${integerJson(balanceCents)}}`
    ))
    .join(',');

  return `{"uniqueEvents":${integerJson(report.uniqueEvents)},`
    + `"duplicateEvents":${integerJson(report.duplicateEvents)},`
    + `"accounts":[${accounts}],`
    + `"netCents":${integerJson(report.netCents)}}\n`;
}
