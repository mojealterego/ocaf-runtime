import assert from 'node:assert/strict';
import { test } from 'node:test';
import { PreviewGate } from '../preview-gate.mjs';

test('only the latest request can be consumed, exactly once', () => {
  const gate = new PreviewGate();
  const first = gate.begin();
  const second = gate.begin();
  assert.notEqual(first, second);
  assert.equal(gate.take(first), false);
  assert.equal(gate.take(second), true);
  assert.equal(gate.take(second), false);
});

test('project changes revoke pending validation without reusing identifiers', () => {
  const gate = new PreviewGate();
  const old = gate.begin();
  gate.invalidate();
  assert.equal(gate.take(old), false);
  const current = gate.begin();
  assert.notEqual(current, old);
  assert.equal(gate.take(old), false);
  assert.equal(gate.take(current), true);
});

test('untyped, missing and forged request identifiers cannot consume a request', () => {
  const gate = new PreviewGate();
  const current = gate.begin();
  for (const invalid of [null, undefined, Number(current), {}, '0', '01', current + 'x']) {
    assert.equal(gate.take(invalid), false);
  }
  assert.equal(gate.take(current), true);
});

test('concurrent requests finishing in reverse order cannot overwrite the newest result', async () => {
  const gate = new PreviewGate();
  const first = gate.begin();
  const second = gate.begin();
  let shown = '';
  const complete = async (id, value) => {
    await Promise.resolve();
    if (gate.take(id)) shown = value;
  };
  await complete(second, 'new project');
  await complete(first, 'old project');
  assert.equal(shown, 'new project');
});

test('a native receipt cannot consume an outstanding cloud request', () => {
  const gate = new PreviewGate();
  const cloud = gate.begin('cloud');
  assert.equal(gate.take(cloud, 'native'), false);
  assert.equal(gate.take(cloud, 'cloud'), true);
  assert.throws(() => gate.begin('unknown'), /transport/);
});
