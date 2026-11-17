import test from 'node:test';
import assert from 'node:assert/strict';
import {parseUniqueJson} from '../strict-json.mjs';

test('duplicate root JSON keys are rejected', () => {
  assert.throws(() => parseUniqueJson('{"tenant_id":"a","tenant_id":"b"}'), /duplicate/i);
});

test('nested duplicate JSON keys are rejected before code/no-code conversion', () => {
  assert.throws(() => parseUniqueJson('{"nodes":[{"id":"safe","kind":"input","kind":"tool"}]}'), /duplicate/i);
});

test('escaped equivalent JSON property names are treated as duplicates', () => {
  assert.throws(() => parseUniqueJson('{"tenant_id":"a","tenant_\\u0069d":"b"}'), /duplicate/i);
});

test('distinct objects may contain the same property without false positives', () => {
  assert.deepEqual(parseUniqueJson('[{"name":"a"},{"name":"b"}]'), [{name:'a'},{name:'b'}]);
});

test('normal JSON remains semantically identical to built-in parsing', () => {
  const source = JSON.stringify({z:[],a:{'\u00e5':'a\\n',list:[false,null,1.25e-4,{'x':-500}]}});
  assert.deepEqual(parseUniqueJson(source), JSON.parse(source));
});

test('empty arrays and objects are valid', () => {
  assert.deepEqual(parseUniqueJson('{"obj":{},"array":[]}'), {obj:{},array:[]});
});

test('invalid syntax fails closed', () => {
  assert.throws(() => parseUniqueJson('{"a":1,}'), /json|syntax/i);
});

test('ambiguity inside an array does not escape detection', () => {
  assert.throws(() => parseUniqueJson('[1,true,{"x":1,"x":2},null]'), /duplicate/i);
});

test('deep nesting is bounded to avoid untrusted stack exhaustion', () => {
  const nested='['.repeat(150)+'0'+']'.repeat(150);
  assert.throws(() => parseUniqueJson(nested), /depth|limit/i);
});

test('oversized JSON is rejected before processing', () => {
  assert.throws(() => parseUniqueJson('"'+'a'.repeat(129*1024)+'"'), /size|limit/i);
});

test('escaped surrogate keys compare by decoded value', () => {
  assert.throws(() => parseUniqueJson('{"\\uD83D\\uDE00":1,"😀":2}'), /duplicate/i);
});

test('offline revision history can exceed one DSL payload within its distinct cap', () => {
  const history = JSON.stringify({history: ['x'.repeat(80_000), 'x'.repeat(80_000)]});
  assert.throws(() => parseUniqueJson(history), /size|limit/i);
  assert.equal(parseUniqueJson(history, {maxBytes: 128 * 1024 * 32}).history.length, 2);
});

test('callers cannot raise JSON size cap beyond the bounded offline limit', () => {
  assert.throws(() => parseUniqueJson('{}', {maxBytes: Number.MAX_SAFE_INTEGER}), /limit|invalid/i);
});

test('AB-JSON-014 rejects non-finite numbers in every position', () => {
  for (const source of ['1e400','-1e400','{"nested":[1e400]}']) {
    assert.throws(() => parseUniqueJson(source), /non-finite/i);
  }
});

test('AB-JSON-015 rejects isolated surrogate code units in keys and values', () => {
  for (const source of ['"\\uD800"','{"\\uDC00":1}','{"a":["\\uD800x"]}', '"'+String.fromCharCode(0xD800)+'"']) {
    assert.throws(() => parseUniqueJson(source), /surrogate/i);
  }
});

test('AB-JSON-016 retains valid Unicode pairs and finite numeric extremes', () => {
  const source = '{"\\uD83D\\uDE00":"\\uD83D\\uDE00","values":[1e308,-1e308,0,5e-324]}';
  assert.deepEqual(parseUniqueJson(source), JSON.parse(source));
});
