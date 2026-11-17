import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { PreviewGate } from '../preview-gate.mjs';

// Exercise actual app handlers with controlled DOM and transport adapters.
// This is a source-level integration regression, not a WebView or live Cloud E2E.
const source = readFileSync(new URL('../app.mjs', import.meta.url), 'utf8');

function harness() {
  const begin = source.indexOf('async function preview()');
  const end = source.indexOf('function download()', begin);
  const listenerBegin = source.indexOf("window.addEventListener('ocaf-rust-validation'");
  const listenerEnd = source.indexOf('\n\nconst el', listenerBegin);
  assert.ok(begin >= 0 && end > begin && listenerBegin >= 0 && listenerEnd > listenerBegin);
  const pending = [], listeners = {}, messages = [], notices = [];
  const rendered = { textContent: '' };
  const state = { dirty: false, project: { project_id: 'first', max_cost_minor: 0 }, workspace: 'local', saved: 1 };
  const cloudClient = { preview() { return new Promise((resolve, reject) => pending.push({resolve, reject})); } };
  const window = {
    addEventListener: (name, cb) => { listeners[name] = cb; },
    ocafHost: { postMessage: raw => messages.push(JSON.parse(raw)) }
  };
  const previewGate = new PreviewGate();
  const plan = { ordered_nodes: ['first'], requested_tools: [], max_cost_minor: 0,
    project_sha256: 'a'.repeat(64), namespace: 'local', effects_executed: false };
  const preview = new Function(
    'state','cloudClient','window','previewGate','TENANT','currentTenant','el','notice','fail',
    'validateProject','encodeDsl','currentId',
    source.slice(listenerBegin, listenerEnd) + '\n' + source.slice(begin, end) + '\nreturn preview;'
  )(state, cloudClient, window, previewGate, 'local',
    () => state.workspace === 'cloud' ? 'tenant' : 'local',
    () => rendered, message => notices.push(message), error => notices.push('FAIL ' + String(error)),
    project => ({order:[project.project_id], tools:[], cost:0}), JSON.stringify, () => state.project.project_id);
  const receive = id => listeners['ocaf-rust-validation']({ detail: JSON.stringify({
    request_id: id, ok: true, plan: JSON.stringify(plan)
  }) });
  return {preview,state,pending,messages,rendered,previewGate,plan,notices,receive};
}

test('actual native handler rejects stale, duplicate and invalidated receipts', async () => {
  const h = harness();
  await h.preview();
  const old = h.messages[0].request_id;
  await h.preview();
  const fresh = h.messages[1].request_id;
  assert.notEqual(old, fresh);
  const initial = h.rendered.textContent;
  h.receive(old);
  assert.equal(h.rendered.textContent, initial);
  h.receive(fresh);
  assert.match(h.rendered.textContent, /Rust UniFFI: inert plan verified/);
  const verified = h.rendered.textContent;
  h.receive(fresh);
  assert.equal(h.rendered.textContent, verified);
  await h.preview();
  const pending = h.messages[2].request_id;
  h.previewGate.invalidate();
  const previous = h.rendered.textContent;
  h.receive(pending);
  assert.equal(h.rendered.textContent, previous);
});

test('actual Cloud preview ignores out-of-order responses and revoked errors', async () => {
  const h = harness();
  h.state.workspace = 'cloud';
  const earlier = h.preview();
  const current = h.preview();
  h.pending[1].resolve({...h.plan, ordered_nodes:['new']});
  await current;
  const displayed = h.rendered.textContent;
  assert.match(displayed, /Graph: new/);
  h.pending[0].resolve({...h.plan, ordered_nodes:['old']});
  await earlier;
  assert.equal(h.rendered.textContent, displayed);
  const revoked = h.preview();
  h.previewGate.invalidate();
  const count = h.notices.length;
  h.pending[2].reject(new Error('stale network failure'));
  await revoked;
  assert.equal(h.notices.length, count);
});
