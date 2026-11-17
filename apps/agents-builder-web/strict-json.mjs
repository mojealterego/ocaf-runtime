/**
 * OCAF Agents Builder (REQ-024, REQ-042): unambiguous bounded JSON input.
 *
 * JavaScript JSON.parse() uses last-key-wins, but Rust/typed contract parsers
 * may reject repeated object keys. Duplicate properties (including Unicode
 * escaped aliases) are rejected before admitting Code DSL or cloud receipts.
 * This does not grant rights, validate DAG semantics or execute agent tools.
 */
const MAX_SOURCE_BYTES = 128 * 1024;
const MAX_NESTING_DEPTH = 64;
const WHITESPACE = new Set([' ', '\t', '\r', '\n']);

/**
 * Parse bounded JSON, rejecting duplicate keys, non-finite numbers and
 * unpaired UTF-16 surrogates in all keys and string values.
 * Returns exactly the JSON.parse value for accepted inputs.
 */
export function parseUniqueJson(source, {maxBytes = MAX_SOURCE_BYTES} = {}) {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > MAX_SOURCE_BYTES * 32) {
    throw new Error('json: invalid size limit');
  }
  if (typeof source !== 'string') throw new Error('json: expected text');
  if (new TextEncoder().encode(source).length > maxBytes) {
    throw new Error('json: input size limit exceeded');
  }
  let parsed;
  try { parsed = JSON.parse(source); }
  catch { throw new Error('json: invalid syntax'); }

  let index = 0;
  const whitespace = () => {
    while (index < source.length && WHITESPACE.has(source[index])) index += 1;
  };
  const stringToken = () => {
    if (source[index] !== '"') throw new Error('json: invalid object key');
    const start = index++;
    while (index < source.length) {
      const char = source[index++];
      if (char === '\\') { index += 1; continue; } // JSON.parse checked escape syntax
      if (char === '"') {
        const decoded = JSON.parse(source.slice(start, index));
        // Unicode scalar values only, matching the Rust JSON boundary.
        for (let i = 0; i < decoded.length; i++) {
          const unit = decoded.charCodeAt(i);
          if (unit >= 0xD800 && unit <= 0xDBFF) {
            const low = decoded.charCodeAt(++i);
            if (!(low >= 0xDC00 && low <= 0xDFFF)) {
              throw new Error('json: unpaired surrogate');
            }
          } else if (unit >= 0xDC00 && unit <= 0xDFFF) {
            throw new Error('json: unpaired surrogate');
          }
        }
        return decoded;
      }
    }
    throw new Error('json: unterminated string');
  };
  const value = depth => {
    if (depth > MAX_NESTING_DEPTH) throw new Error('json: nesting depth limit exceeded');
    whitespace();
    if (source[index] === '"') { stringToken(); return; }
    if (source[index] === '{') {
      index += 1;
      whitespace();
      const seen = new Set();
      if (source[index] === '}') { index += 1; return; }
      for (;;) {
        const key = stringToken();
        if (seen.has(key)) throw new Error('json: duplicate object key');
        seen.add(key);
        whitespace();
        if (source[index] !== ':') throw new Error('json: expected colon');
        index += 1;
        value(depth + 1);
        whitespace();
        const next = source[index++];
        if (next === '}') return;
        if (next !== ',') throw new Error('json: expected separator');
        whitespace();
      }
    }
    if (source[index] === '[') {
      index += 1;
      whitespace();
      if (source[index] === ']') { index += 1; return; }
      for (;;) {
        value(depth + 1);
        whitespace();
        const next = source[index++];
        if (next === ']') return;
        if (next !== ',') throw new Error('json: expected separator');
      }
    }
    // JSON.parse() has already verified all primitive syntax and delimiters.
    const start = index;
    while (index < source.length &&
           !WHITESPACE.has(source[index]) &&
           ![',', ']', '}'].includes(source[index])) index += 1;
    const first = source[start];
    if ((first === '-' || (first >= '0' && first <= '9')) &&
        !Number.isFinite(Number(source.slice(start, index)))) {
      throw new Error('json: non-finite number');
    }
  };
  value(0);
  whitespace();
  if (index !== source.length) throw new Error('json: unexpected data after value');
  return parsed;
}
