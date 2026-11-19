import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { PreviewGate } from '../preview-gate.mjs';

// Source-level integration: actual application handlers, controlled transport and DOM.
const source = readFileSync(new URL('../app.mjs', import.meta.url), 'utf8');

function harness() {
  const a = source.indexOf('function requireCloud()');
  const b = source.indexOf('async function saveCloudProject()', a);
  const c = source.indexOf('function invalidatePreview()');
  const d = source.indexOf('function render()', c);
  assert.ok(a >= 0 && b > a && c >= 0 && d > c);
  const pending = [], notices = [];
  const state = { workspace:'cloud', cloud:{tenant:'tenant', head:null, pending:false},
    project:{project_id:'original', revision:1}, saved:1, dirty:false };
  const cloudClient = { load(tenant, id) {
    return new Promise((resolve,reject) => pending.push({tenant,id,resolve,reject}));
  } };
  const fields = {'project-id':{value:'first'}, 'preview-output':{textContent:''}};
  const previewGate = new PreviewGate(), cloudSelectionGate = new PreviewGate(), importGate = new PreviewGate();
  const window = { confirm: () => true };
  class CloudApiError extends Error { constructor(status) { super('HTTP '+status); this.status=status; } }
  const el = id => fields[id];
  let invalidate;
  const result = new Function('state','cloudClient','cloudSaveInFlight','cloudSessionGeneration',
    'el','window','notice','newProject','CloudApiError','previewGate','cloudSelectionGate','importGate','render',
    source.slice(c,d)+'\n'+source.slice(a,b)+'\nreturn {openCloudProject,newCloudProject,invalidatePreview};'
  )(state,cloudClient,false,1,el,window,m=>notices.push(m),
    (tenant,id)=>({tenant_id:tenant,project_id:id,revision:1}),
    CloudApiError,previewGate,cloudSelectionGate,importGate,()=>invalidate());
  invalidate = result.invalidatePreview;
  const open = id => { fields['project-id'].value=id; return result.openCloudProject(); };
  const create = id => { fields['project-id'].value=id; return result.newCloudProject(); };
  const reply = i => pending[i].resolve({project:{project_id:pending[i].id,revision:1},head:{revision:1}});
  return {state,pending,notices,window,open,create,reply,invalidate,CloudApiError};
}

test('latest Cloud open wins even when older response arrives last', async () => {
  const h=harness(), a=h.open('old'), b=h.open('new');
  h.reply(1); await b; h.reply(0); await a;
  assert.equal(h.state.project.project_id,'new');
});
test('older new-project lookup cannot replace a more recent open', async () => {
  const h=harness(), a=h.create('draft'), b=h.open('existing');
  h.reply(1); await b; h.pending[0].reject(new h.CloudApiError(404)); await a;
  assert.equal(h.state.project.project_id,'existing');
  assert.equal(h.state.cloud.pending,false);
});
test('older failed request does not surface a stale error', async () => {
  const h=harness(), a=h.open('old'), b=h.open('new');
  h.reply(1); await b; h.pending[0].reject(new h.CloudApiError(503)); await a;
  assert.equal(h.state.project.project_id,'new');
});
test('editing and workspace invalidation prevent pending results overwriting drafts', async () => {
  const h=harness(), a=h.open('remote');
  h.state.dirty=true; h.invalidate(); h.reply(0); await a;
  assert.equal(h.state.project.project_id,'original');
  assert.equal(h.state.dirty,true);
  h.state.dirty=false;
  const b=h.open('next'); h.state.workspace='local'; h.invalidate();
  h.state.workspace='cloud'; h.invalidate(); h.reply(1); await b;
  assert.equal(h.state.project.project_id,'original');
});
test('declined discard preserves unapplied Code edits and sends no request', async () => {
  const h=harness(); h.state.dirty=true; h.window.confirm=()=>false;
  await h.open('remote'); await h.create('new');
  assert.equal(h.pending.length,0); assert.equal(h.state.dirty,true);
});
test('current successful open clears dirty state after explicit discard', async () => {
  const h=harness(); h.state.dirty=true;
  const a=h.open('remote'); h.reply(0); await a;
  assert.equal(h.state.project.project_id,'remote'); assert.equal(h.state.dirty,false);
});
test('current lookup failure propagates and a current 404 creates a draft', async () => {
  const h=harness(), a=h.open('unavailable');
  h.pending[0].reject(new h.CloudApiError(503));
  await assert.rejects(a,/503/);
  const b=h.create('draft'); h.pending[1].reject(new h.CloudApiError(404)); await b;
  assert.equal(h.state.project.project_id,'draft'); assert.equal(h.state.cloud.pending,true);
});
