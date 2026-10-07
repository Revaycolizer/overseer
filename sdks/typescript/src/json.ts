import type { IdInput } from './types.js';

/** Validate before any number conversion: rounded Snowflake IDs cannot be recovered. */
export function idToDecimal(value: IdInput): string {
  if (typeof value === 'number' && !Number.isSafeInteger(value)) {
    throw new RangeError('Record IDs must be decimal strings, bigint, or safe integers');
  }
  const text = String(value);
  if (!/^-?(0|[1-9]\d*)$/.test(text)) throw new TypeError('Invalid decimal record ID');
  const integer = BigInt(text);
  if (integer < -9223372036854775808n || integer > 9223372036854775807n) {
    throw new RangeError('Record ID exceeds signed 64-bit range');
  }
  return text;
}

/** Preserve integer tokens before JSON.parse has a chance to round them. */
export function decodeJson(text: string): unknown {
  const preserved = text.replace(/"(?:\\.|[^"\\])*"|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/g, token => {
    if (/^-?\d+$/.test(token) && !Number.isSafeInteger(Number(token))) return JSON.stringify(token);
    return token;
  });
  const result = JSON.parse(preserved, (key, value: unknown) => {
    if ((key === 'id' || key.endsWith('_id')) && (typeof value === 'number' || typeof value === 'string') && /^-?\d+$/.test(String(value))) {
      return idToDecimal(value);
    }
    return value;
  }) as unknown;
  // Creation endpoints return a bare i64 ID.
  return typeof result === 'number' && Number.isInteger(result) ? String(result) : result;
}

/** Send decimal IDs as JSON integer tokens, as required by the Rust REST handlers. */
export function encodeJson(value: unknown, idArray = false): string {
  const normalized = JSON.stringify(value, (key, item: unknown) => {
    if (typeof item === 'bigint') return idToDecimal(item);
    if (typeof item === 'number' && (key === 'id' || key.endsWith('_id'))) idToDecimal(item);
    return item;
  });
  if (normalized === undefined) throw new TypeError('Body must be JSON serializable');
  // Walk valid JSON tokens, replacing only numeric ID strings, never message contents.
  return normalized.replace(/("(?:\\.|[^"\\])*"\s*:\s*)?"(?:\\.|[^"\\])*"/g, (token, prefix: string | undefined) => {
    if (!prefix && !idArray) return token;
    if (prefix) {
      const key = JSON.parse(prefix.slice(0, prefix.lastIndexOf(':')).trim()) as string;
      if (key !== 'id' && !key.endsWith('_id')) return token;
    }
    const item = JSON.parse(prefix ? token.slice(prefix.length) : token) as string;
    return /^-?\d+$/.test(item) ? (prefix ?? '') + idToDecimal(item) : token;
  });
}
