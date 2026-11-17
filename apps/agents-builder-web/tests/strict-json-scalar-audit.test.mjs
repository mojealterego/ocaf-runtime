import test from 'node:test';
import assert from 'node:assert/strict';
import {parseUniqueJson} from '../strict-json.mjs';

test('AB-JSON-AUDIT-001 rejects every isolated surrogate in keys and values', () => {
  for (let x = 0xd800; x <= 0xdfff; x++) {
    const s = '"\\u' + x.toString(16) + '"';
    assert.throws(() => parseUniqueJson(s), /surrogate/);
    assert.throws(() => parseUniqueJson('{' + s + ':0}'), /surrogate/);
  }
});

test('AB-JSON-AUDIT-002 retains sampled supplementary Unicode scalars', () => {
  for (let x = 0x10000; x <= 0x10ffff; x += 127) {
    const s = JSON.stringify({[String.fromCodePoint(x)]: String.fromCodePoint(x)});
    assert.deepEqual(parseUniqueJson(s), JSON.parse(s));
  }
});

test('AB-JSON-AUDIT-003 checks exponent boundaries and nested invalid input', () => {
  for (let x = -340; x <= 340; x++) {
    const s = '1e' + String(x);
    if (Number.isFinite(Number(s))) assert.equal(parseUniqueJson(s), Number(s));
    else assert.throws(() => parseUniqueJson(s), /non-finite/);
  }
  for (const s of ['{"a":1,"\\u0061":2}', '{"x":[{"y":1,"y":2}]}', '[1e999]', '{"a":"\\udc00"}']) {
    assert.throws(() => parseUniqueJson(s));
  }
});
